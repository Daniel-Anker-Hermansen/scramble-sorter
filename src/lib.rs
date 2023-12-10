extern crate console_error_panic_hook;
use std::{fmt::Write as _, io::{Read, Cursor, Write, Seek}, collections::HashMap};
use wasm_bindgen::prelude::*;
use std::panic;
use js_sys::{Uint8Array,ArrayBuffer};

use zip::{ZipWriter, write::FileOptions, ZipArchive};

fn read_passcodes(zip: &mut ZipArchive<impl Read + Seek>, password: &str) -> Result<String, JsError> {
    let passcode_file_name = zip.file_names().find(|file| file.contains("Passcodes")).unwrap().to_string();
    let mut passcodes_file = zip.by_name_decrypt(&passcode_file_name, password.as_bytes())??;
    let mut passcodes = String::new();
    passcodes_file.read_to_string(&mut passcodes)?;
    Ok(passcodes)
}


#[wasm_bindgen]
pub async fn run(buffer: ArrayBuffer, competition_name: &str, password: &str) -> Result<js_sys::Uint8Array, JsError> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let uint8_array = Uint8Array::new(&buffer);
    let mut in_memory_file = Cursor::new(uint8_array.to_vec());

    let mut zip = zip::ZipArchive::new(&mut in_memory_file)?;
    let passcodes = read_passcodes(&mut zip, password)?;
    let scrambles_file_name = zip.file_names().find(|file| file.contains("PDFs")).unwrap().to_string();
    let mut scrambles_file = zip.by_name_decrypt(&scrambles_file_name, password.as_bytes())??;
    let mut cursor = Cursor::new(Vec::new());
    scrambles_file.read_to_end(cursor.get_mut())?;

    let activity_name_to_passcode: HashMap<String, String> = passcodes.lines()
        .skip(9)
        .map(|pair| {
            let (event, passcode) = pair.split_once(":").unwrap();
            (event.trim().to_string(), passcode.trim().to_string())
        })
        .collect();

    let mut scrambles_zip = zip::ZipArchive::new(&mut cursor)?;
    
    let filenames: Vec<_> = scrambles_zip.file_names()
        .filter(|filename| !filename.contains("Fewest"))
        .map(|filename| {
            let (event, rest) = filename.split(".").next().unwrap().split_once("Round").unwrap();
            let mut iter = rest.split_whitespace();
            let event = event.trim();
            let mut round = iter.next();
            let mut attempt = None;
            while let Some(val) = iter.next() {
                match val {
                    "Round" => round = iter.next(),
                    "Attempt" => attempt = iter.next(),
                    _ => ()
                }
            }
            ((event.to_string(), round.map(str::to_string), attempt.map(str::to_string)), filename.to_string())
        })
        .collect();
    
    let wcif = get_wcif(competition_name).await?;
    let activity_codes = extract_round_order_from_json(wcif);

    let mut indices = filenames.into_iter()
        .map(|(e, filename)| {
            let equiv = equiv_assignment_code(&e);
            let index = activity_codes.iter().position(|code| code == &equiv).ok_or(JsError::new(&format!("Unable to match '{}' to schduled activity", filename)));
            index.map(|i| (i, filename))
        })
        .collect::<Result<Vec<_>, _>>()?;

    indices.sort_by(|(index_0, group_0), (index_1, group_1)|
        index_0.cmp(index_1)
            .then_with(|| group_0.len().cmp(&group_1.len())
            .then_with(|| group_0.cmp(group_1))));

    let mut buffer = Cursor::new(Vec::new());
    let mut sorted_scramble_zip = zip::ZipWriter::new(&mut buffer);
    let mut passcodes = String::new();
    for (_, filename) in &indices {
        let (individual_scramble_file_name, _) = filename.split_once(".").expect("all scramble files have a period");
        let passcode_for_pdf = &activity_name_to_passcode[individual_scramble_file_name];
        writeln!(&mut passcodes, "{}: {}", individual_scramble_file_name, passcode_for_pdf).expect("writing to string is infallible");
    }

    for (idx, (_, filename)) in indices.into_iter().enumerate() {
        let individual_scramble_zip = scrambles_zip.by_name(&filename)?;
        sorted_scramble_zip.raw_copy_file_rename(individual_scramble_zip, format!("{:04}: {}", idx, filename))?;
    }

    sorted_scramble_zip.finish()?;
    drop(sorted_scramble_zip);

    let mut final_zip_buffer = Cursor::new(Vec::new());
    let mut final_zip = ZipWriter::new(&mut final_zip_buffer);
    final_zip.start_file("scrambles.zip", FileOptions::default())?;
    final_zip.write_all(&buffer.into_inner().as_slice())?;
    final_zip.start_file("passcodes.txt", FileOptions::default())?;
    final_zip.write_all(&passcodes.as_bytes())?;
    final_zip.finish()?;
    drop(final_zip);

    Ok(js_sys::Uint8Array::from(final_zip_buffer.into_inner().as_slice()))
}

fn equiv_assignment_code(event: &(String, Option<String>, Option<String>)) -> String {
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
    let attempt_string = match &event.2 {
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

async fn get_wcif(comp_name: &str) -> Result<serde_json::Value,JsError> {
    let wcif_response = reqwest::get(format!("https://api.worldcubeassociation.org/competitions/{}/wcif/public", comp_name)).await?;
    if wcif_response.status().is_success(){
            let wcif = wcif_response.text().await.unwrap();
            Ok(serde_json::from_str(&wcif).unwrap())
        }
    else{
        // Err()
        Err(JsError::new("The competition ID supplied gives an error. Check that you wrote it correctly and that the WCA site is working.")) 
    }
    
}
