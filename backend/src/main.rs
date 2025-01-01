#[macro_use]
extern crate rocket;

use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket_cors::CorsOptions;
use rand::Rng;

// Struttura per la richiesta dell'utente per /roll
#[derive(Serialize, Deserialize)]
struct DiceRequest {
    num_dadi: usize,
}

// Struttura per la richiesta per /roll_with_reroll
#[derive(Serialize, Deserialize)]
struct RollWithRerollRequest {
    num_dadi: usize,
    rilancia_uno: bool,
}

// Struttura per la richiesta per /reroll
#[derive(Serialize, Deserialize)]
struct RerollRequest {
    risultati: Vec<u8>,
}

// Struttura per la risposta dell'API
#[derive(Serialize, Deserialize)]
struct DiceResponse {
    risultati: Vec<u8>,
    raises: usize,
    combinazioni: Vec<Vec<u8>>,
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

// Trova il massimo numero di Raises (successi)
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

// Trova tutte le combinazioni valide di dadi che sommano a target
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

// Endpoint POST per calcolare i Raises
#[post("/roll", format = "json", data = "<dice_request>")]
fn roll_dice(dice_request: Json<DiceRequest>) -> Result<Json<DiceResponse>, rocket::http::Status> {
    if dice_request.num_dadi == 0 {
        return Err(rocket::http::Status::BadRequest);
    }
    if dice_request.num_dadi > 100 {
        return Err(rocket::http::Status::BadRequest);
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
fn roll_with_reroll(request: Json<RollWithRerollRequest>) -> Result<Json<RollWithRerollResponse>, rocket::http::Status> {
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
fn reroll_dice(reroll_request: Json<RerollRequest>) -> Result<Json<DiceResponse>, rocket::http::Status> {
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

// Configura e avvia Rocket
#[launch]
fn rocket() -> _ {
    let cors = CorsOptions::default()
        .to_cors()
        .expect("CORS configuration failed");

    rocket::build()
        .mount("/", routes![roll_dice, reroll_dice, roll_with_reroll])
        .attach(cors)
}
