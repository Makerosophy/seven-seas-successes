use rand::Rng;
use std::io::{self, Write};

//TODO: Aggiungere backend Rocket
/// Trova il massimo numero di Raises (successi)
fn massimizza_raises(dadi: Vec<u8>) -> (usize, Vec<Vec<u8>>) {
    let mut raises = 0;
    let mut combinazioni_ottimali = Vec::new();
    let mut restanti = dadi.clone();

    while !restanti.is_empty() {
        let mut combinazioni = Vec::new();
        let mut combinazione_corrente = Vec::new();

        // Trova tutte le combinazioni valide
        trova_combinazioni(&restanti, 10, 0, &mut combinazione_corrente, &mut combinazioni);

        if !combinazioni.is_empty() {
            // Scegli la combinazione migliore (quella che libera più dadi)
            let combinazione_migliore = combinazioni
                .iter()
                .max_by_key(|combinazione| combinazione.len())
                .unwrap()
                .clone();

            raises += 1;
            combinazioni_ottimali.push(combinazione_migliore.clone());

            // Rimuovi i dadi utilizzati
            for valore in combinazione_migliore {
                if let Some(pos) = restanti.iter().position(|&x| x == valore) {
                    restanti.remove(pos);
                }
            }
        } else {
            break; // Nessuna combinazione valida trovata
        }
    }

    (raises, combinazioni_ottimali)
}

/// Trova tutte le combinazioni valide di dadi che sommano a target
fn trova_combinazioni(
    dadi: &Vec<u8>,
    target: i32,
    start: usize,
    combinazione: &mut Vec<u8>,
    combinazioni: &mut Vec<Vec<u8>>,
) {
    // Caso base: se raggiungiamo o superiamo il target, la combinazione è valida
    if target <= 0 {
        combinazioni.push(combinazione.clone());
        return;
    }

    // Caso base: fine dell'array
    if start >= dadi.len() {
        return;
    }

    // Include il dado corrente
    combinazione.push(dadi[start]);
    trova_combinazioni(
        dadi,
        target - dadi[start] as i32,
        start + 1,
        combinazione,
        combinazioni,
    );
    combinazione.pop();

    // Esclude il dado corrente
    trova_combinazioni(dadi, target, start + 1, combinazione, combinazioni);
}

fn main() {
    let mut rng = rand::thread_rng();

    loop {
        // Chiedi all'utente quanti dadi vuole lanciare
        print!("Quanti dadi da 10 facce vuoi lanciare? ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let num_dadi: usize = match input.trim().parse() {
            Ok(n) if n > 0 => n,
            _ => {
                println!("Per favore, inserisci un numero valido maggiore di 0.");
                continue;
            }
        };

        // Lancia i dadi
        let mut risultati: Vec<u8> = (0..num_dadi)
            .map(|_| rng.gen_range(1..=10))
            .collect();

        // Ordina i risultati in ordine decrescente
        risultati.sort_unstable_by(|a, b| b.cmp(a));

        // Mostra i risultati
        println!("Risultati dei dadi (ordinati): {:?}", risultati);

        // Regola aggiunta: permetti il rilancio di un dado con valore 1
        if risultati.contains(&1) {
            print!("Vuoi rilanciare un dado con risultato 1? (s/n): ");
            io::stdout().flush().unwrap();
            let mut scelta = String::new();
            io::stdin().read_line(&mut scelta).unwrap();

            if scelta.trim().eq_ignore_ascii_case("s") {
                if let Some(pos) = risultati.iter().position(|&x| x == 1) {
                    let nuovo_tiro = rng.gen_range(1..=10);
                    println!(
                        "Hai rilanciato un dado con risultato 1 ed ottenuto: {}",
                        nuovo_tiro
                    );

                    // Aggiorna il dado con il nuovo valore
                    risultati[pos] = nuovo_tiro;

                    // Riordina i risultati in ordine decrescente
                    risultati.sort_unstable_by(|a, b| b.cmp(a));

                    println!("Nuovi risultati dei dadi (ordinati): {:?}", risultati);
                }
            }
        }

        // Calcola Raises massimizzando le combinazioni valide
        let (raises, combinazioni) = massimizza_raises(risultati.clone());
        println!("Numero di Raises: {}", raises);
        println!("Combinazioni valide (pari o superiori a 10): {:?}", combinazioni);

        // Chiedi se l'utente vuole lanciare di nuovo
        print!("Vuoi lanciare di nuovo? (s/n): ");
        io::stdout().flush().unwrap();
        let mut scelta = String::new();
        io::stdin().read_line(&mut scelta).unwrap();
        if !scelta.trim().eq_ignore_ascii_case("s") {
            break;
        }
    }
}
