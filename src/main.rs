use std::{fmt::Write as _, io::{Read, Cursor, Write}, collections::HashMap};

#[tokio::main]
async fn main() {
    let mut reader = std::fs::File::open("scrambles.zip").unwrap();
    let mut zip = zip::ZipArchive::new(&mut reader).unwrap();
    let passcode_file_name = zip.file_names().find(|file| file.contains("Passcodes")).unwrap().to_string();
    let mut passcodes_file = zip.by_name_decrypt(&passcode_file_name, b"hej").unwrap().unwrap();
    let mut passcodes = String::new();
    passcodes_file.read_to_string(&mut passcodes).unwrap();
    drop(passcodes_file);
    let scrambles_file_name = zip.file_names().find(|file| file.contains("PDFs")).unwrap().to_string();
    let mut scrambles_file = zip.by_name_decrypt(&scrambles_file_name, b"hej").unwrap().unwrap();
    let mut scrambles = Vec::new();
    scrambles_file.read_to_end(&mut scrambles).unwrap();
    let mut cursor = Cursor::new(scrambles);

    let map: HashMap<String, String> = passcodes.lines()
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
    
    let wcif = get_wcif("BjerringbroOpen2023").await;
    let codes = extract_round_order_from_json(wcif);

    let mut indices: Vec<_> = filenames.into_iter()
        .map(|(e, filename)| {
            let equiv = equiv_assignment_code(&e);
            let index = codes.iter().position(|code| code == &equiv).unwrap();
            (index, e.3, filename)
        })
        .collect();
    indices.sort_by_key(|(i, group, _)| (*i, group.clone()));
    
    let mut buffer = Cursor::new(Vec::new());
    let mut new_zip = zip::ZipWriter::new(&mut buffer);
    let mut passcodes = String::new();
    for (idx, (_, _, filename)) in indices.into_iter().enumerate() {
        let zip_file = scrambles_zip.by_name(&filename).unwrap();
        new_zip.raw_copy_file_rename(zip_file, format!("{:04}: {}", idx, filename)).unwrap();
        let (without_zip, _) = filename.split_once(".").unwrap();
        dbg!(&without_zip);
        writeln!(&mut passcodes, "{}: {}", without_zip, map[without_zip]);
    }
    let mut file = std::fs::File::create("sorted_scrambles.zip").unwrap();
    new_zip.finish().unwrap();
    drop(new_zip);
    file.write_all(&buffer.into_inner()).unwrap();
    println!("{}", passcodes);
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
    let mut activities: Vec<&serde_json::Value> = json["schedule"]["venues"].as_array().unwrap()
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
