use rand::Rng;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use web_sys::HtmlInputElement;
use wasm_logger;

// -------------- Strutture dati ---------------- //

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

// -------------- Logica di calcolo ---------------- //

// Trova il massimo numero di Raises (successi)
fn massimizza_raises(dadi: &[u8]) -> (usize, Vec<Vec<u8>>) {
    let mut raises = 0;
    let mut combinazioni_ottimali = Vec::new();
    let mut restanti = dadi.to_vec();

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
    dadi: &[u8],
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

// Lancia semplicemente `num_dadi` e calcola i raises
fn roll_dice(num_dadi: usize) -> Option<DiceResponse> {
    // Validazioni
    if num_dadi == 0 || num_dadi > 100 {
        return None;
    }

    let mut rng = rand::thread_rng();
    let mut risultati: Vec<u8> = (0..num_dadi).map(|_| rng.gen_range(1..=10)).collect();
    risultati.sort_unstable_by(|a, b| b.cmp(a));

    let (raises, combinazioni) = massimizza_raises(&risultati);

    Some(DiceResponse {
        risultati,
        raises,
        combinazioni,
    })
}

// Lancia `num_dadi`, e se richiesto, rilancia un dado da 1
fn roll_with_reroll(num_dadi: usize, rilancia_uno: bool) -> Option<RollWithRerollResponse> {
    // Validazioni
    if num_dadi == 0 || num_dadi > 100 {
        return None;
    }

    let mut rng = rand::thread_rng();
    let mut risultati: Vec<u8> = (0..num_dadi).map(|_| rng.gen_range(1..=10)).collect();
    let risultati_originali = risultati.clone();

    let mut rilanciato = None;
    if rilancia_uno {
        if let Some(index) = risultati.iter().position(|&x| x == 1) {
            let nuovo_valore = rng.gen_range(1..=10);
            risultati[index] = nuovo_valore;
            rilanciato = Some(nuovo_valore);
        }
    }

    risultati.sort_unstable_by(|a, b| b.cmp(a));
    let (raises, combinazioni) = massimizza_raises(&risultati);

    Some(RollWithRerollResponse {
        risultati_originali,
        rilanciato,
        risultati_aggiornati: risultati,
        raises,
        combinazioni,
    })
}

// Rilancia un dado da 1 (se presente) in un vettore di risultati
fn reroll_dice(mut risultati: Vec<u8>) -> Option<DiceResponse> {
    let mut rng = rand::thread_rng();
    if let Some(index) = risultati.iter().position(|&x| x == 1) {
        risultati[index] = rng.gen_range(1..=10);
    } else {
        // Se non c'è un 1 da rilanciare, ritorno None
        return None;
    }

    risultati.sort_unstable_by(|a, b| b.cmp(a));
    let (raises, combinazioni) = massimizza_raises(&risultati);

    Some(DiceResponse {
        risultati,
        raises,
        combinazioni,
    })
}

// -------------- Componente Yew ---------------- //

#[function_component(App)]
fn app() -> Html {
    // Stato per l'input "numero di dadi"
    let num_dadi = use_state(|| 5);
    // Stato per l'input "rilancia automaticamente un 1"
    let rilancia_uno = use_state(|| false);
    // Stato che contiene il risultato dell'ultimo lancio
    let results = use_state(|| None::<RollWithRerollResponse>);
    // Stato che indica se siamo "in caricamento"
    // (Non è più veramente necessario senza chiamate HTTP, ma lo manteniamo per UI/UX)
    let loading = use_state(|| false);

    // Funzione per resettare l'app allo stato iniziale
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

    // Handler per il lancio dei dadi (con o senza reroll)
    let handle_roll = {
        let results = results.clone();
        let loading = loading.clone();
        let num_dadi_value = *num_dadi;
        let rilancia_uno_value = *rilancia_uno;

        Callback::from(move |_| {
            loading.set(true);

            // Simuliamo un "async" con spawn_local, ma in realtà chiamiamo la funzione diretta
            let results_clone = results.clone();
            let loading_clone = loading.clone();

            spawn_local(async move {
                let maybe_result = if rilancia_uno_value {
                    // Chiamata a roll_with_reroll
                    roll_with_reroll(num_dadi_value, true).map(|r| r)
                } else {
                    // Chiamata a roll_dice
                    roll_dice(num_dadi_value).map(|dr| RollWithRerollResponse {
                        risultati_originali: dr.risultati.clone(),
                        rilanciato: None,
                        risultati_aggiornati: dr.risultati,
                        raises: dr.raises,
                        combinazioni: dr.combinazioni,
                    })
                };

                if let Some(res) = maybe_result {
                    results_clone.set(Some(res));
                }
                loading_clone.set(false);
            });
        })
    };

    // Handler per "rilancia un dado da 1" (se presente)
    let handle_reroll = {
        let results = results.clone();

        Callback::from(move |_| {
            if let Some(current) = &*results {
                let risultati_clonati = current.risultati_aggiornati.clone();
                let originali_clonati = current.risultati_originali.clone();
                let results_clonati = results.clone();

                spawn_local(async move {
                    if let Some(data) = reroll_dice(risultati_clonati) {
                        results_clonati.set(Some(RollWithRerollResponse {
                            risultati_originali: originali_clonati,
                            rilanciato: data.risultati.last().copied(),
                            risultati_aggiornati: data.risultati,
                            raises: data.raises,
                            combinazioni: data.combinazioni,
                        }));
                    }
                });
            }
        })
    };

    html! {
        <div class="container">
            <h1>{ "Gestione successi in 7th Sea (senza backend)" }</h1>

            <div class="input-container">
                <label>{ "Seleziona il numero di dadi da lanciare:" }</label>
                <input
                    type="number"
                    value={num_dadi.to_string()}
                    oninput={Callback::from(move |e: InputEvent| {
                        if let Ok(value) = e.target_unchecked_into::<HtmlInputElement>()
                            .value()
                            .parse::<usize>()
                        {
                            num_dadi.set(value);
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
                            let rilancia_uno = rilancia_uno.clone();
                            move |_| rilancia_uno.set(!*rilancia_uno)
                        })}
                    />
                    { "Rilancia automaticamente il primo dado da 1" }
                </label>
            </div>

            <button
                class="roll-button"
                onclick={handle_roll}
                disabled={*loading}
            >
                { "Roll" }
            </button>

            <button class="reset-button" onclick={reset_app}>
                { "Reset" }
            </button>

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
                                            <p>{ format!("Ultimo dado rilanciato: {}", rilanciato) }</p>
                                            <p>{ format!("Dadi aggiornati: {:?}", result.risultati_aggiornati) }</p>
                                        </>
                                    }
                                } else {
                                    html! {
                                        <p>{ format!("Dadi lanciati: {:?}", result.risultati_aggiornati) }</p>
                                    }
                                }
                            }
                            <p class="success-count">{ format!("Successi (Raises): {}", result.raises) }</p>
                            <p>{ format!("Combinazioni: {:?}", result.combinazioni) }</p>
                            <button
                                class="reroll-button"
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
    // Inizializza il logger per debug su console
    wasm_logger::init(wasm_logger::Config::default());
    // Avvio dell'app Yew
    yew::Renderer::<App>::new().render();
}
