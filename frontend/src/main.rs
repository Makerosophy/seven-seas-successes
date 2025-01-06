use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use gloo_net::http::Request;
use web_sys::HtmlInputElement;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DiceRequest {
    num_dadi: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RollWithRerollRequest {
    num_dadi: usize,
    rilancia_uno: bool,
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoginRequest {
    user_id: String,
}

#[function_component(App)]
fn app() -> Html {
    let num_dadi = use_state(|| 5);
    let rilancia_uno = use_state(|| false);
    let results = use_state(|| None::<RollWithRerollResponse>);
    let loading = use_state(|| false);
    let user_name = use_state(|| String::new());
    let token = use_state(|| None::<String>);

    // Funzione per resettare lo stato dell'app
    let reset_app = {
        let num_dadi = num_dadi.clone();
        let rilancia_uno = rilancia_uno.clone();
        let results = results.clone();
        Callback::from(move |_| {
            num_dadi.set(5);
            rilancia_uno.set(false);
            results.set(None);
        })
    };

    // Gestione del login per ottenere il token
    let handle_login = {
        let user_name = user_name.clone();
        let token = token.clone();

        Callback::from(move |_| {
            let user_name = user_name.clone();
            let token = token.clone();

            spawn_local(async move {
                let body = LoginRequest {
                    user_id: (*user_name).clone(),
                };

                match Request::post("http://localhost:8000/login")
                    .header("Content-Type", "application/json")
                    .body(serde_json::to_string(&body).unwrap())
                    .unwrap()
                    .send()
                    .await
                {
                    Ok(response) => {
                        if let Ok(jwt) = response.text().await {
                            token.set(Some(jwt));
                        } else {
                            log::error!("Errore nel parsing del token");
                        }
                    }
                    Err(err) => log::error!("Errore durante il login: {:?}", err),
                }
            });
        })
    };

    // Funzione per lanciare i dadi
    let handle_roll = {
        let results = results.clone();
        let loading = loading.clone();
        let num_dadi = *num_dadi;
        let rilancia_uno = *rilancia_uno;
        let token = token.clone();

        Callback::from(move |_| {
            if token.is_none() {
                log::error!("Effettua il login prima di lanciare i dadi.");
                return;
            }

            let results = results.clone();
            let loading = loading.clone();
            let token = token.clone();

            spawn_local(async move {
                loading.set(true);

                let url = if rilancia_uno {
                    "http://localhost:8000/roll_with_reroll"
                } else {
                    "http://localhost:8000/roll"
                };

                let request_body = if rilancia_uno {
                    serde_json::to_string(&RollWithRerollRequest { num_dadi, rilancia_uno }).unwrap()
                } else {
                    serde_json::to_string(&DiceRequest { num_dadi }).unwrap()
                };

                match Request::post(url)
                    .header(
                        "Authorization",
                        &format!(
                            "Bearer {}",
                            token.as_ref().unwrap()
                        ),
                    )
                    .header("Content-Type", "application/json")
                    .body(request_body)
                    .unwrap()
                    .send()
                    .await
                {
                    Ok(response) => {
                        if rilancia_uno {
                            if let Ok(data) = response.json::<RollWithRerollResponse>().await {
                                results.set(Some(data));
                            }
                        } else {
                            if let Ok(data) = response.json::<DiceResponse>().await {
                                results.set(Some(RollWithRerollResponse {
                                    risultati_originali: data.risultati.clone(),
                                    rilanciato: None,
                                    risultati_aggiornati: data.risultati,
                                    raises: data.raises,
                                    combinazioni: data.combinazioni,
                                }));
                            }
                        }
                    }
                    Err(err) => log::error!("Errore durante la richiesta: {:?}", err),
                }
                loading.set(false);
            });
        })
    };

    // Funzione per rilanciare i dadi da 1
    let handle_reroll = {
        let results = results.clone();
        let token = token.clone();

        Callback::from(move |_| {
            if token.is_none() {
                log::error!("Effettua il login prima di rilanciare un dado.");
                return;
            }

            if let Some(result) = (*results).clone() {
                let risultati_clonati = result.risultati_aggiornati.clone();
                let results_clonati = results.clone();
                let originali_clonati = result.risultati_originali.clone();
                let token = token.clone();

                spawn_local(async move {
                    match Request::post("http://localhost:8000/reroll")
                        .header(
                            "Authorization",
                            &format!(
                                "Bearer {}",
                                token.as_ref().unwrap()
                            ),
                        )
                        .header("Content-Type", "application/json")
                        .json(&serde_json::json!({ "risultati": risultati_clonati }))
                        .unwrap()
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if let Ok(data) = response.json::<DiceResponse>().await {
                                results_clonati.set(Some(RollWithRerollResponse {
                                    risultati_originali: originali_clonati,
                                    rilanciato: Some(data.risultati.last().copied().unwrap_or(0)),
                                    risultati_aggiornati: data.risultati,
                                    raises: data.raises,
                                    combinazioni: data.combinazioni,
                                }));
                            }
                        }
                        Err(err) => log::error!("Errore durante il rilancio: {:?}", err),
                    }
                });
            }
        })
    };

    html! {
        <div class="container">
            <h1>{ "Gestione successi in 7th Sea" }</h1>
            <div class="login-container">
                <label>{ "Inserisci il tuo nome utente:" }</label>
                <input
                    type="text"
                    value={(*user_name).clone()}
                    oninput={Callback::from(move |e: InputEvent| {
                        let input: HtmlInputElement = e.target_unchecked_into();
                        user_name.set(input.value());
                    })}
                />
                <button class="login-button" onclick={handle_login}>{ "Conferma Nome" }</button>
            </div>
            <div class="input-container">
                <label>{ "Seleziona il numero di dadi da lanciare:" }</label>
                <input type="number"
                    value={num_dadi.to_string()}
                    oninput={Callback::from(move |e: InputEvent| {
                        if let Ok(value) = e.target_unchecked_into::<HtmlInputElement>().value().parse::<usize>() {
                            num_dadi.set(value);
                        }
                    })}
                />
            </div>
            <div class="checkbox-container">
                <label>
                    <input type="checkbox"
                        checked={*rilancia_uno}
                        onchange={Callback::from({
                            let rilancia_uno = rilancia_uno.clone();
                            move |_| rilancia_uno.set(!*rilancia_uno)
                        })}
                    />
                    { "Rilancia automaticamente" }
                </label>
            </div>
            <button class="roll-button" onclick={handle_roll} disabled={*loading}>{ "Roll" }</button>
            <button class="reset-button" onclick={reset_app}>{ "Reset" }</button>
            {
                if let Some(result) = &*results {
                    html! {
                        <div class="results">
                            <h2>{ "Risultati" }</h2>
                            <p>{ format!("Dadi originali: {:?}", result.risultati_originali) }</p>
                            {
                                if let Some(rilanciato) = result.rilanciato {
                                    html! {
                                        <>
                                            <p>{ format!("Rilanciato: {}", rilanciato) }</p>
                                            <p>{ format!("Dadi aggiornati: {:?}", result.risultati_aggiornati) }</p>
                                        </>
                                    }
                                } else {
                                    html! {}
                                }
                            }
                            <p class="success-count">{ format!("Successi (Raises): {}", result.raises) }</p>
                            <p>{ format!("Combinazioni: {:?}", result.combinazioni) }</p>
                            <button class="reroll-button"
                                onclick={handle_reroll}
                                disabled={*rilancia_uno || !result.risultati_aggiornati.contains(&1)}
                            >
                                { "Ritira un dado da 1" }
                            </button>
                        </div>
                    }
                } else {
                    html! { <p>{ "Pronto a iniziare la tua avventura?" }</p> }
                }
            }
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
