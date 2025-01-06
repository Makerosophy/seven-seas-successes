#[macro_use]
extern crate rocket;

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::outcome::Outcome;
use rocket_cors::CorsOptions;
use rand::Rng;
use dotenv::dotenv;
use std::env;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, errors::Result as JwtResult};
use chrono::{Utc, Duration};

// Struttura per i claim del token JWT
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[derive(Serialize, Deserialize)]
// Implementazione di un request guard per l'autenticazione
struct Auth {
    user_id: String,
}

#[derive(Serialize, Deserialize)]
struct LoginRequest {
    user_id: String,
}


// Strutture per le richieste e le risposte
#[derive(Serialize, Deserialize)]
struct DiceRequest {
    num_dadi: usize,
}

#[derive(Serialize, Deserialize)]
struct DiceResponse {
    risultati: Vec<u8>,
    raises: usize,
    combinazioni: Vec<Vec<u8>>,
}

// Struttura per la richiesta per /roll_with_reroll
#[derive(Serialize, Deserialize)]
struct RollWithRerollRequest {
    num_dadi: usize,
    rilancia_uno: bool,
}

// Struttura per la risposta di /roll_with_reroll
#[derive(Serialize, Deserialize)]
struct RollWithRerollResponse {
    risultati_originali: Vec<u8>,
    rilanciato: Option<u8>,
    risultati_aggiornati: Vec<u8>,
    raises: usize,
    combinazioni: Vec<Vec<u8>>,
}

// Struttura per la richiesta per /reroll
#[derive(Serialize, Deserialize)]
struct RerollRequest {
    risultati: Vec<u8>,
}

// Funzione per creare un token JWT
fn create_jwt(user_id: &str, secret: &str) -> JwtResult<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("Errore nella creazione della data di scadenza")
        .timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref()))
}

// Funzione per verificare un token JWT
fn verify_jwt(token: &str, secret: &str) -> JwtResult<Claims> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default()).map(|data| data.claims)
}


#[rocket::async_trait]
impl<'r> FromRequest<'r> for Auth {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let keys: Vec<_> = request.headers().get("Authorization").collect();
        if keys.len() != 1 {
            return Outcome::Error((Status::Unauthorized, ()));
        }

        let token = keys[0].trim_start_matches("Bearer ").to_string();
        let secret = match env::var("SECRET_KEY") {
            Ok(key) => key,
            Err(_) => return Outcome::Error((Status::InternalServerError, ())),
        };

        match verify_jwt(&token, &secret) {
            Ok(claims) => Outcome::Success(Auth { user_id: claims.sub }),
            Err(_) => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}


fn massimizza_raises(dadi: Vec<u8>) -> (usize, Vec<Vec<u8>>) {
    let mut raises = 0;
    let mut combinazioni_ottimali = Vec::new();
    let mut restanti = dadi.clone();

    while !restanti.is_empty() {
        let mut combinazioni = Vec::new();
        let mut combinazione_corrente = Vec::new();

        trova_combinazioni(&restanti, 10, 0, &mut combinazione_corrente, &mut combinazioni);

        if !combinazioni.is_empty() {
            let combinazione_migliore = combinazioni
                .iter()
                .max_by_key(|combinazione| combinazione.len())
                .unwrap()
                .clone();

            raises += 1;
            combinazioni_ottimali.push(combinazione_migliore.clone());

            for valore in combinazione_migliore {
                if let Some(pos) = restanti.iter().position(|&x| x == valore) {
                    restanti.remove(pos);
                }
            }
        } else {
            break;
        }
    }

    (raises, combinazioni_ottimali)
}

fn trova_combinazioni(
    dadi: &Vec<u8>,
    target: i32,
    start: usize,
    combinazione: &mut Vec<u8>,
    combinazioni: &mut Vec<Vec<u8>>,
) {
    if target <= 0 {
        combinazioni.push(combinazione.clone());
        return;
    }
    if start >= dadi.len() {
        return;
    }
    combinazione.push(dadi[start]);
    trova_combinazioni(
        dadi,
        target - dadi[start] as i32,
        start + 1,
        combinazione,
        combinazioni,
    );
    combinazione.pop();
    trova_combinazioni(dadi, target, start + 1, combinazione, combinazioni);
}

// Endpoint protetto per il lancio dei dadi
#[post("/roll", format = "json", data = "<dice_request>")]
async fn roll_dice(_auth: Auth, dice_request: Json<DiceRequest>) -> Result<Json<DiceResponse>, Status> {
    if dice_request.num_dadi == 0 || dice_request.num_dadi > 100 {
        return Err(Status::BadRequest);
    }

    let mut rng = rand::thread_rng();
    let mut risultati: Vec<u8> = (0..dice_request.num_dadi)
        .map(|_| rng.gen_range(1..=10))
        .collect();
    risultati.sort_unstable_by(|a, b| b.cmp(a));

    let (raises, combinazioni) = massimizza_raises(risultati.clone());

    Ok(Json(DiceResponse {
        risultati,
        raises,
        combinazioni,
    }))
}

// Endpoint POST per rilanciare opzionalmente un dado da 1
#[post("/roll_with_reroll", format = "json", data = "<request>")]
async fn roll_with_reroll(_auth: Auth, request: Json<RollWithRerollRequest>) -> Result<Json<RollWithRerollResponse>, Status> {
    if request.num_dadi == 0 {
        return Err(rocket::http::Status::BadRequest);
    }
    if request.num_dadi > 100 {
        return Err(rocket::http::Status::BadRequest);
    }

    let mut rng = rand::thread_rng();
    let mut risultati: Vec<u8> = (0..request.num_dadi)
        .map(|_| rng.gen_range(1..=10))
        .collect();

    let risultati_originali = risultati.clone();
    let mut rilanciato = None;

    if request.rilancia_uno {
        if let Some(index) = risultati.iter().position(|&x| x == 1) {
            let nuovo_valore = rng.gen_range(1..=10);
            risultati[index] = nuovo_valore;
            rilanciato = Some(nuovo_valore);
        }
    }

    risultati.sort_unstable_by(|a, b| b.cmp(a));

    let (raises, combinazioni) = massimizza_raises(risultati.clone());

    Ok(Json(RollWithRerollResponse {
        risultati_originali,
        rilanciato,
        risultati_aggiornati: risultati,
        raises,
        combinazioni,
    }))
}

// Endpoint POST per rilanciare il primo dado con valore 1
#[post("/reroll", format = "json", data = "<reroll_request>")]
async fn reroll_dice(_auth: Auth, reroll_request: Json<RerollRequest>) -> Result<Json<DiceResponse>, Status> {
    let mut risultati = reroll_request.risultati.clone();
    let mut rng = rand::thread_rng();

    if let Some(index) = risultati.iter().position(|&x| x == 1) {
        risultati[index] = rng.gen_range(1..=10);
    } else {
        return Err(rocket::http::Status::BadRequest);
    }

    risultati.sort_unstable_by(|a, b| b.cmp(a));

    let (raises, combinazioni) = massimizza_raises(risultati.clone());

    Ok(Json(DiceResponse {
        risultati,
        raises,
        combinazioni,
    }))
}


// Endpoint pubblico per ottenere un token JWT
#[post("/login", format = "json", data = "<login_request>")]
async fn login(login_request: Json<LoginRequest>) -> Result<String, Status> {
    let user_id = &login_request.user_id;
    let secret = match env::var("SECRET_KEY") {
        Ok(key) => key,
        Err(_) => return Err(Status::InternalServerError),
    };

    match create_jwt(&user_id, &secret) {
        Ok(token) => Ok(token),
        Err(_) => Err(Status::InternalServerError),
    }
}

// Configurazione e avvio dell'applicazione Rocket
#[launch]
fn rocket() -> _ {
    dotenv().ok();

    let cors = CorsOptions::default()
        .to_cors()
        .expect("CORS configuration failed");

    rocket::build()
        .mount("/", routes![roll_dice, login, roll_with_reroll, reroll_dice])
        .attach(cors)
}