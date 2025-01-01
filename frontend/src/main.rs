use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use gloo_net::http::Request;
use web_sys::HtmlInputElement; // Import necessario per gestire gli input

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

#[function_component(App)]
fn app() -> Html {
    let num_dadi = use_state(|| 5);
    let rilancia_uno = use_state(|| false);
    let results = use_state(|| None::<RollWithRerollResponse>);
    let loading = use_state(|| false);

    let handle_roll = {
        let results = results.clone();
        let loading = loading.clone();
        let num_dadi = *num_dadi;
        let rilancia_uno = *rilancia_uno;

        Callback::from(move |_| {
            let results = results.clone();
            let loading = loading.clone();

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

    let handle_reroll = {
        let results = results.clone();
        Callback::from(move |_| {
            if let Some(result) = (*results).clone() {
                // Cloniamo i dati necessari per avere il lifetime `'static`
                let risultati_clonati = result.risultati_aggiornati.clone();
                let results_clonati = results.clone();
                let originali_clonati = result.risultati_originali.clone();

                spawn_local(async move {
                    match Request::post("http://localhost:8000/reroll")
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
                                    rilanciato: None,
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
            <h1>{ "Calcola i successi in 7th Seas" }</h1>
            <div>
                <label>{ "Numero di dadi:" }</label>
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
                        onchange={Callback::from(move |_| rilancia_uno.set(!*rilancia_uno))}
                    />
                    { "Rilancia uno dei dadi con valore 1 automaticamente" }
                </label>
            </div>
            <button onclick={handle_roll} disabled={*loading}>{ "Lancia i dadi" }</button>
            {
                if let Some(result) = &*results {
                    html! {
                        <div class="results">
                            <h2>{ "Risultati" }</h2>
                            <p>{ format!("Dadi originali: {:?}", result.risultati_originali) }</p>
                            {
                                if let Some(rilanciato) = result.rilanciato {
                                    html! { <p>{ format!("Rilanciato: {}", rilanciato) }</p> }
                                } else {
                                    html! { <p>{ "Nessun dado rilanciato" }</p> }
                                }
                            }
                            <p>{ format!("Dadi aggiornati: {:?}", result.risultati_aggiornati) }</p>
                            <p>{ format!("Successi (Raises): {}", result.raises) }</p>
                            <p>{ format!("Combinazioni: {:?}", result.combinazioni) }</p>
                            <button onclick={handle_reroll}>{ "Rilancia un dado da 1" }</button>
                        </div>
                    }
                } else {
                    html! { <p>{ "Premi il pulsante per lanciare i dadi." }</p> }
                }
            }
        </div>
    }    
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
