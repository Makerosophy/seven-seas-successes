use rand::Rng;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use wasm_logger;
use web_sys::{console, HtmlInputElement};
use yew::functional::function_component;
use yew::prelude::*;
use yew::prelude::use_mut_ref;
use yew_websocket::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

/* ------------------ Strutture & Messaggi per la Chat ------------------ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub message: String,
}

/// Messaggi che il client invia al server (es. aggiungere un messaggio di chat)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum ClientMessage {
    AddMessage(ChatMessage),
}

/// Messaggi inviati dal server al client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
enum ServerMessage {
    FullHistory(Vec<ChatMessage>),
    System(String),
}

/* ------------------ Strutture & Logica Dadi ------------------ */

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DiceResponse {
    risultati: Vec<u8>,
    raises: usize,
    combinazioni: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RollWithRerollResponse {
    risultati_originali: Vec<u8>,
    rilanciato: Option<u8>,
    risultati_aggiornati: Vec<u8>,
    raises: usize,
    combinazioni: Vec<Vec<u8>>,
}

/// Calcolo dei raises
fn massimizza_raises(dadi: &[u8]) -> (usize, Vec<Vec<u8>>) {
    let mut raises = 0;
    let mut combo_ottimali = Vec::new();
    let mut restanti = dadi.to_vec();

    while !restanti.is_empty() {
        let mut possibili = Vec::new();
        let mut combo_corrente = Vec::new();
        trova_combinazioni(&restanti, 10, 0, &mut combo_corrente, &mut possibili);

        if !possibili.is_empty() {
            let migliore = possibili
                .iter()
                .max_by_key(|c| c.len())
                .unwrap()
                .clone();
            raises += 1;
            combo_ottimali.push(migliore.clone());

            for v in migliore {
                if let Some(pos) = restanti.iter().position(|&x| x == v) {
                    restanti.remove(pos);
                }
            }
        } else {
            break;
        }
    }

    (raises, combo_ottimali)
}

fn trova_combinazioni(
    dadi: &[u8],
    target: i32,
    start: usize,
    combo: &mut Vec<u8>,
    risultato: &mut Vec<Vec<u8>>,
) {
    if target <= 0 {
        risultato.push(combo.clone());
        return;
    }
    if start >= dadi.len() {
        return;
    }
    // includi
    combo.push(dadi[start]);
    trova_combinazioni(dadi, target - dadi[start] as i32, start + 1, combo, risultato);
    combo.pop();
    // salta
    trova_combinazioni(dadi, target, start + 1, combo, risultato);
}

/// Lancia `num_dadi` e ordina i risultati in decrescente
fn roll_dice(num_dadi: usize) -> Option<DiceResponse> {
    if num_dadi == 0 || num_dadi > 100 {
        return None;
    }

    let mut rng = rand::thread_rng();
    let mut ris: Vec<u8> = (0..num_dadi).map(|_| rng.gen_range(1..=10)).collect();
    ris.sort_unstable_by(|a, b| b.cmp(a));

    let (r, combo) = massimizza_raises(&ris);
    Some(DiceResponse {
        risultati: ris,
        raises: r,
        combinazioni: combo,
    })
}

/// Lancia `num_dadi`, e se richiesto rilancia un `1`
fn roll_with_reroll(num_dadi: usize, rilancia_uno: bool) -> Option<RollWithRerollResponse> {
    if num_dadi == 0 || num_dadi > 100 {
        return None;
    }

    let mut rng = rand::thread_rng();
    let mut ris: Vec<u8> = (0..num_dadi).map(|_| rng.gen_range(1..=10)).collect();
    let originali = ris.clone();

    let mut rilanciato = None;
    if rilancia_uno {
        if let Some(idx) = ris.iter().position(|&x| x == 1) {
            let nuovo = rng.gen_range(1..=10);
            ris[idx] = nuovo;
            rilanciato = Some(nuovo);
        }
    }

    ris.sort_unstable_by(|a, b| b.cmp(a));
    let (r, combo) = massimizza_raises(&ris);

    Some(RollWithRerollResponse {
        risultati_originali: originali,
        rilanciato,
        risultati_aggiornati: ris,
        raises: r,
        combinazioni: combo,
    })
}

/// Rerolla un dado `1`, riordina e ricalcola i raises
fn reroll_dice(mut ris: Vec<u8>) -> Option<DiceResponse> {
    let mut rng = rand::thread_rng();
    if let Some(idx) = ris.iter().position(|&x| x == 1) {
        ris[idx] = rng.gen_range(1..=10);
    } else {
        return None;
    }
    ris.sort_unstable_by(|a, b| b.cmp(a));
    let (r, combo) = massimizza_raises(&ris);

    Some(DiceResponse {
        risultati: ris,
        raises: r,
        combinazioni: combo,
    })
}

/* ---------------------- COMPONENTE PRINCIPALE YEW ---------------------- */

#[function_component(App)]
fn app() -> Html {
    // ---------- Stati: dadi ----------
    let num_dadi = use_state(|| 5);
    let rilancia_uno = use_state(|| false);
    let results = use_state(|| None::<RollWithRerollResponse>);
    let loading = use_state(|| false);

    // ---------- Stati: chat e WebSocket ----------
    let username = use_state(|| "".to_string());
    let chat_messages = use_state(|| Vec::<ChatMessage>::new());
    let ws_status_text = use_state(|| "Non connesso".to_string());
    let ws_task = use_mut_ref(|| None::<WebSocketTask>);
    let is_connected = use_state(|| false); // stato "sono collegato?"

    // ---------- onmessage ----------
    let onmessage = {
        let chat_messages = chat_messages.clone();
        Callback::from(move |res: Result<String, anyhow::Error>| {
            match res {
                Ok(txt) => {
                    match serde_json::from_str::<ServerMessage>(&txt) {
                        Ok(server_msg) => {
                            match server_msg {
                                ServerMessage::FullHistory(log_vec) => {
                                    console::log_1(&"(FullHistory) ricevuto".into());
                                    // invertiamo => il più recente index 0
                                    let mut reversed = log_vec;
                                    reversed.reverse();
                                    chat_messages.set(reversed);
                                }
                                ServerMessage::System(sys_str) => {
                                    console::log_1(&format!("(System) => {}", sys_str).into());
                                    let mut new_list = (*chat_messages).clone();
                                    new_list.insert(0, ChatMessage {
                                        username: "SYSTEM".to_string(),
                                        message: sys_str,
                                    });
                                    chat_messages.set(new_list);
                                }
                            }
                        }
                        Err(_) => {
                            console::log_1(&format!("Msg non parseabile come ServerMessage: {}", txt).into());
                        }
                    }
                }
                Err(err) => {
                    console::error_1(&format!("onmessage error: {:?}", err).into());
                }
            }
        })
    };

    // ---------- onnotification ----------
    let onnotification = {
        let ws_status_text = ws_status_text.clone();
        let is_connected = is_connected.clone();
        Callback::from(move |status: WebSocketStatus| {
            match status {
                WebSocketStatus::Opened => {
                    ws_status_text.set("Collegato!".into());
                    is_connected.set(true);
                    console::log_1(&"WS aperto".into());
                }
                WebSocketStatus::Closed => {
                    ws_status_text.set("Connessione chiusa".into());
                    is_connected.set(false);
                    console::log_1(&"WS chiuso".into());
                }
                WebSocketStatus::Error => {
                    ws_status_text.set("Errore nella connessione".into());
                    is_connected.set(false);
                    console::log_1(&"WS errore".into());
                }
            }
        })
    };

    // ---------- connect_ws ----------
    let connect_ws = {
        let ws_task = ws_task.clone();
        let onmsg = onmessage.clone();
        let onnote = onnotification.clone();
        let is_connected = is_connected.clone();

        Callback::from(move |_| {
            if *is_connected {
                console::log_1(&"Sei già connesso!".into());
                return;
            }
            let mut ws_ref = ws_task.borrow_mut();
            if ws_ref.is_none() {
                match WebSocketService::connect_text(
                    "wss://dice-server-qxze.onrender.com/ws/",
                    onmsg.clone(),
                    onnote.clone(),
                ) {
                    Ok(task) => {
                        *ws_ref = Some(task);
                        console::log_1(&"Connessione WebSocket avviata".into());
                    }
                    Err(e) => {
                        console::error_1(&format!("Connessione WS fallita: {:?}", e).into());
                    }
                }
            }
        })
    };

    // ---------- disconnect_ws ----------
    let disconnect_ws = {
        let ws_task = ws_task.clone();
        let is_connected = is_connected.clone();
        let ws_status_text = ws_status_text.clone();

        Callback::from(move |_| {
            // se c'è un Some(WebSocketTask), lo prendo e lo droppo => la connessione si chiude
            if ws_task.borrow().is_some() {
                ws_task.borrow_mut().take(); 
                // L'oggetto WebSocketTask viene droppato qui => disconnessione

                console::log_1(&"WS disconnesso manualmente".into());
                is_connected.set(false);
                ws_status_text.set("Connessione chiusa".into());
            }
        })
    };

    // ---------- invio messaggi in chat ----------
    let send_message = {
        let ws_task = ws_task.clone();
        let username = username.clone();
        Callback::from(move |contenuto: String| {
            let uname = (*username).clone();
            if uname.is_empty() {
                console::log_1(&"Devi inserire uno username prima di inviare messaggi".into());
                return;
            }

            if let Some(ref mut task) = *ws_task.borrow_mut() {
                // Costruiamo un ClientMessage
                let msg = ClientMessage::AddMessage(ChatMessage {
                    username: uname,
                    message: contenuto,
                });
                if let Ok(json_str) = serde_json::to_string(&msg) {
                    task.send(json_str);
                }
            }
        })
    };

    // ---------- reset_app ----------
    let reset_app = {
        let nd = num_dadi.clone();
        let ru = rilancia_uno.clone();
        let rs = results.clone();
        Callback::from(move |_| {
            nd.set(5);
            ru.set(false);
            rs.set(None);
        })
    };

    // ---------- handle_roll ----------
    let handle_roll = {
        let results_handle = results.clone();
        let user_handle = username.clone();
        let loading_flag = loading.clone();
        let do_send = send_message.clone();
        let n_dadi = *num_dadi;
        let r_auto = *rilancia_uno;

        Callback::from(move |_| {
            let results2 = results_handle.clone();
            let user2 = (*user_handle).clone();
            let loading2 = loading_flag.clone();
            let send2 = do_send.clone();

            spawn_local(async move {
                if user2.is_empty() {
                    console::log_1(&"Inserisci username prima di rollare".into());
                    return;
                }
                loading2.set(true);

                let maybe = if r_auto {
                    roll_with_reroll(n_dadi, true)
                } else {
                    roll_dice(n_dadi).map(|dr| RollWithRerollResponse {
                        risultati_originali: dr.risultati.clone(),
                        rilanciato: None,
                        risultati_aggiornati: dr.risultati,
                        raises: dr.raises,
                        combinazioni: dr.combinazioni,
                    })
                };

                if let Some(res) = maybe {
                    results2.set(Some(res.clone()));

                    // Includiamo anche la lista di combinazioni
                    let combos_str = format!("{:?}", res.combinazioni);
                    let text = format!(
                        "{} ha tirato {} dadi: {:?} (raises: {}) | Combinazioni: {}",
                        user2, n_dadi, res.risultati_aggiornati, res.raises, combos_str
                    );
                    send2.emit(text);
                }
                loading2.set(false);
            });
        })
    };

    // ---------- handle_reroll ----------
    let handle_reroll = {
        let results_handle = results.clone();
        let user_handle = username.clone();
        let do_send = send_message.clone();
        let r_auto = *rilancia_uno;

        Callback::from(move |_| {
            let results2 = results_handle.clone();
            let user2 = (*user_handle).clone();
            let send2 = do_send.clone();

            spawn_local(async move {
                if user2.is_empty() {
                    console::log_1(&"Inserisci username prima di rerollare".into());
                    return;
                }
                if let Some(cur) = &*results2 {
                    if r_auto {
                        console::log_1(&"Reroll automatico già attivo, non serve reroll manuale".into());
                        return;
                    }
                    let dati_agg = cur.risultati_aggiornati.clone();
                    let orig = cur.risultati_originali.clone();

                    if let Some(dr) = reroll_dice(dati_agg) {
                        let ris_clone = dr.risultati.clone();
                        let combos_str = format!("{:?}", dr.combinazioni);

                        results2.set(Some(RollWithRerollResponse {
                            risultati_originali: orig,
                            rilanciato: ris_clone.last().copied(),
                            risultati_aggiornati: ris_clone.clone(),
                            raises: dr.raises,
                            combinazioni: dr.combinazioni,
                        }));

                        let text = format!(
                            "{} ha rerollato un dado da 1. Nuovi risultati: {:?} (raises: {}) | Combinazioni: {}",
                            user2, ris_clone, dr.raises, combos_str
                        );
                        send2.emit(text);
                    }
                }
            });
        })
    };

    // Leggiamo lo stato "connesso"
    let connected = *is_connected;

    // ---------- RENDER UI ----------
    html! {
        <div class="container">
            <h1>{ "Gestione successi in 7th Sea" }</h1>

            {
                // Se non connesso => inserisci username e "Connetti"
                // Se connesso => mostra "Disconnetti"
                if !connected {
                    html! {
                        <div class="input-container">
                            <label>{ "Giocatore" }</label>
                            <input
                                type="text"
                                value={(*username).clone()}
                                oninput={Callback::from({
                                    let un = username.clone();
                                    move |e: InputEvent| {
                                        un.set(e.target_unchecked_into::<HtmlInputElement>().value());
                                    }
                                })}
                            />
                            <button class="roll-button"
                                onclick={connect_ws}
                                disabled={(*username).is_empty()}>
                                { "Connetti" }
                            </button>
                        </div>
                    }
                } else {
                    html! {
                        <div class="input-container">
                            <label>{ "Sei connesso come:" }</label>
                            <p>{ (*username).clone() }</p>
                            <button class="reset-button" onclick={disconnect_ws}>
                                { "Disconnetti" }
                            </button>
                        </div>
                    }
                }
            }

            <div class="input-container">
                <label>{ "Numero di dadi" }</label>
                <input
                    type="number"
                    value={num_dadi.to_string()}
                    oninput={Callback::from({
                        let nd = num_dadi.clone();
                        move |e: InputEvent| {
                            if let Ok(value) = e.target_unchecked_into::<HtmlInputElement>()
                                .value()
                                .parse::<usize>() {
                                nd.set(value);
                            }
                        }
                    })}
                />
            </div>

            <div class="checkbox-container">
                <label>
                    <input
                        type="checkbox"
                        checked={*rilancia_uno}
                        onchange={Callback::from({
                            let ru = rilancia_uno.clone();
                            move |_| {
                                ru.set(!*ru)
                            }
                        })}
                    />
                    { " Rilancia il primo 1 automaticamente" }
                </label>
            </div>

            // Pulsanti di "roll" e "reset"
            <button
                class="roll-button"
                onclick={handle_roll}
                disabled={*loading || (*username).is_empty() || !connected}
            >
                { "Roll" }
            </button>
            <button
                class="reset-button"
                onclick={reset_app}
            >
                { "Reset" }
            </button>

            // Se abbiamo un risultato, mostriamo i dettagli
            {
                if let Some(r) = &*results {
                    html! {
                        <div class="results">
                            <h2>{ "Ultimo Tiro" }</h2>
                            <p>{ format!("DADI ORIGINALI: {:?}", r.risultati_originali) }</p>
                            {
                                if let Some(ril) = r.rilanciato {
                                    html! {
                                        <>
                                            <p>{ format!("Esito dell'ultimo rilancio: {}", ril) }</p>
                                            <p>{ format!("DADI AGGIORNATI: {:?}", r.risultati_aggiornati) }</p>
                                        </>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                            <p class="success-count">{ format!("Successi (Raises): {}", r.raises) }</p>
                            <p>{ format!("Combinazioni: {:?}", r.combinazioni) }</p>

                            <button
                                class="reroll-button"
                                onclick={handle_reroll}
                                disabled={*rilancia_uno
                                    || !r.risultati_aggiornati.contains(&1)
                                    || (*username).is_empty()
                                    || !connected}
                            >
                                { "Ritira un dado da 1" }
                            </button>
                        </div>
                    }
                } else {
                    html! { <p>{ "Pronto a iniziare la tua avventura?" }</p> }
                }
            }

            // Sezione log
            <div class="container">
                <h2>{ "Log dei tiri:" }</h2>
                <ul>
                {
                    // I messaggi più recenti in index=0 => li stampiamo in quell'ordine
                    for (*chat_messages).iter().enumerate().map(|(_i, msg)| {
                        html! {
                            <li>{ &msg.message }</li>
                        }
                    })
                }
                </ul>
            </div>
        </div>
    }
}

fn main() {
    // Abilita logger per debug
    wasm_logger::init(wasm_logger::Config::default());
    // Avvia l'app Yew
    yew::Renderer::<App>::new().render();
}
