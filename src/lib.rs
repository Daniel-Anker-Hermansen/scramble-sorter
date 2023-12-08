extern crate console_error_panic_hook;
use std::{fmt::Write as _, io::{Read, Cursor, Write}, collections::HashMap};
use wasm_bindgen::prelude::*;
use std::panic;
use js_sys::{Uint8Array,ArrayBuffer};

use zip::{read::ZipFile, ZipArchive};


#[wasm_bindgen]
pub async fn run(buffer: ArrayBuffer, competition_name: &str) ->(js_sys::Uint8Array){
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    println!("hi from rust!");
    let uint8_array = Uint8Array::new(&buffer);
    let my_vec = uint8_array.to_vec();

    // let mut reader = std::fs::File::create("foo.txt").unwrap();
    let mut in_memory_file = Cursor::new(my_vec);
    // reader.write_all(&my_vec).unwrap();

    // let mut reader = std::fs::File::open("scrambles.zip").expect("could not open scramble zip");
    println!("found scramble file");
    let mut zip = zip::ZipArchive::new(&mut in_memory_file).unwrap();
    let passcode_file_name = zip.file_names().find(|file| file.contains("Passcodes")).unwrap().to_string();
    let mut passcodes_file = zip.by_name_decrypt(&passcode_file_name, b"hej").unwrap().unwrap();
    let mut passcodes = String::new();
    passcodes_file.read_to_string(&mut passcodes).unwrap();
    drop(passcodes_file);
    let scrambles_file_name = zip.file_names().find(|file| file.contains("PDFs")).unwrap().to_string();
    //TODO Fix passcode
    let mut scrambles_file = zip.by_name_decrypt(&scrambles_file_name, b"hej").unwrap().unwrap();
    let mut scrambles = Vec::new();
    scrambles_file.read_to_end(&mut scrambles).unwrap();
    let mut cursor = Cursor::new(scrambles);

    let activity_name_to_passcode: HashMap<String, String> = passcodes.lines()
        .skip(9)
        .map(|pair| {
            let (event, passcode) = pair.split_once(":").unwrap();
            (event.trim().to_string(), passcode.trim().to_string())
        })
        .collect();

    let mut scrambles_zip = zip::ZipArchive::new(&mut cursor).unwrap();
    
    let filenames: Vec<_> = scrambles_zip.file_names()
        .filter(|filename| !filename.contains("Fewest"))
        .map(|filename| {
            let (event, rest) = filename.split(".").next().unwrap().split_once("Round").unwrap();
            let mut iter = rest.split_whitespace();
            let event = event.trim();
            let mut round = iter.next();
            let mut group = None;
            let mut attempt = None;
            while let Some(val) = iter.next() {
                let next = iter.next();
                match val {
                    "Round" => round = next,
                    "Attempt" => attempt = next,
                    "Group" => group = next,
                    _ => ()
                }
            }
            ((event.to_string(), round.map(str::to_string), group.map(str::to_string), attempt.map(str::to_string)), filename.to_string())
        })
        .collect();
    
    let wcif = get_wcif(competition_name).await;
    let activity_codes = extract_round_order_from_json(wcif);

    let mut indices: Vec<_> = filenames.into_iter()
        .map(|(e, filename)| {
            let equiv = equiv_assignment_code(&e);
            let index = activity_codes.iter().position(|code| code == &equiv).unwrap();
            (index, e.3, filename)
        })
        .collect();
    indices.sort_by_key(|(i, group, _)| (*i, group.clone()));

    let mut buffer = Cursor::new(Vec::new());
    let mut sorted_scramble_zip = zip::ZipWriter::new(&mut buffer);
    let mut passcodes = String::new();
    let mut passcode_failures = Vec::new();
    for (idx, (_, _, filename)) in indices.into_iter().enumerate() {
        let individual_scramble_zip = scrambles_zip.by_name(&filename).unwrap();
        sorted_scramble_zip.raw_copy_file_rename(individual_scramble_zip, format!("{:04}: {}", idx, filename)).unwrap();
        let (individual_scramble_file_name, _) = filename.split_once(".").unwrap();
        let passcode_for_pdf = &activity_name_to_passcode[individual_scramble_file_name];
        let result = writeln!(&mut passcodes, "{}: {}", individual_scramble_file_name, passcode_for_pdf);
        match result {
            Ok(_) => {}
            Err(err) => {
                passcode_failures.push(format!("Failed getting passcode for: {}. Error: {}", individual_scramble_file_name, err));
            }
        }
    }

    // let mut sorted_scramble_file = std::fs::File::create("sorted_scrambles.zip").unwrap();
    sorted_scramble_zip.finish().unwrap();
    drop(sorted_scramble_zip);
    // sorted_scramble_file.write_all(&buffer.into_inner()).unwrap();
    // let mut sorted_passcode_file = std::fs::File::create("sorted_passcodes.txt").unwrap();
    // sorted_passcode_file.write_all(passcodes.as_bytes()).unwrap();
    let mut serialized_data = Vec::new();
    serialized_data.write_all(&buffer.into_inner()).unwrap();

    let sorted_scrambles = js_sys::Uint8Array::from(serialized_data.as_slice());
    // (passcodes,sorted_scrambles)
    sorted_scrambles
    
}

fn equiv_assignment_code(event: &(String, Option<String>, Option<String>, Option<String>)) -> String {
    let event_str = match event.0.as_str() {
        "3x3x3" => "333",
        "2x2x2" => "222",
        "4x4x4" => "444",
        "5x5x5" => "555",
        "6x6x6" => "666",
        "7x7x7" => "777",
        "3x3x3 One-Handed" => "333oh",
        "3x3x3 Blindfolded" => "333bf",
        "3x3x3 Multiple Blindfolded" => "333mbf",
        "Skewb" =>"skewb",
        "Pyraminx" => "pyram",
        "Clock" => "clock",
        "Megaminx" => "minx",
        "Square-1" => "sq1",
        "4x4x4 Blindfolded" => "444bf",
        "5x5x5 Blindfolded" => "555bf",
        _ => unreachable!(),
    };
    let round_string = match &event.1 {
        Some(a) => format!("-r{}", a),
        None => "".to_string(),
    };
    let attempt_string = match &event.3 {
        Some(a) => format!("-a{}", a),
        None => "".to_string(),
    };
    format!("{}{}{}", event_str, round_string, attempt_string)
}

fn extract_round_order_from_json(json: serde_json::Value) -> Vec<String> {
    let mut activities: Vec<&serde_json::Value> = json["schedule"]["venues"].as_array().expect("failed to parse response from WCA Api")
        .into_iter()
        .flat_map(|venue| venue["rooms"].as_array().unwrap()
                  .into_iter().flat_map(|room| room["activities"].as_array().unwrap()))
        .filter(|activity| !activity["activityCode"].as_str().unwrap().starts_with("o"))
        .collect();
    activities.sort_by_key(|activity| activity["startTime"].as_str().unwrap());
    activities.into_iter().map(|activity| activity["activityCode"].as_str().unwrap().to_owned()).collect()
}

async fn get_wcif(comp_name: &str) -> serde_json::Value {
    let wcif = reqwest::get(format!("https://api.worldcubeassociation.org/competitions/{}/wcif/public", comp_name))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    serde_json::from_str(&wcif).unwrap()
}
