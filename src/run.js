import init, {run} from "./scramble_wasm.js";

console.log('stared first js file');
let elem = document.getElementById('submit_button');
elem.addEventListener('click',run_sorting);

const wasm = init();

export async function handleFile() {
    const fileInput = document.getElementById('scrambles');
    const compid = document.getElementById('compid').value;
    console.log(compid)
    const file = fileInput.files[0];
    console.log("found file")
    if (file) {
        // Convert the file to an ArrayBuffer

        const arrayBuffer = await readFileAsync(file);
        console.log("js read file");
        console.log("ArrayBuffer Length:", arrayBuffer.byteLength);
        // Pass the ArrayBuffer to the Rust Wasm module
        const result = await wasm.then(()=>run(arrayBuffer,compid));
        // update_elements(result);

        // Handle the result from Rust if needed
        console.log(result);

        downloadFile(result, 'example.zip');
    } else {
        console.error("No file selected");
    }
}

function downloadFile(data, fileName) {
    // const blob = new Blob([data], { type: mimeType });
    const mimeType = 'application/zip';  // Set the MIME type to "application/zip"
    const blob = new Blob([data], { type: mimeType });
    const url = window.URL.createObjectURL(blob);

    const a = document.createElement('a');
    a.href = url;
    a.download = fileName;

    // Append the anchor to the body and trigger a click event
    document.body.appendChild(a);
    a.click();

    // Remove the anchor from the body
    document.body.removeChild(a);

    // Revoke the URL
    window.URL.revokeObjectURL(url);
}

export async function readFileAsync(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result);
        reader.onerror = reject;
        reader.readAsArrayBuffer(file);
    });
}

export function run_sorting(){
    console.log('button clicked');
    let file_thing = handleFile();
    // let memo = wasm.then(()=>run()).then(update_elements);
}

function update_elements(passcodes){
    console.log(passcodes);
    document.getElementById("scrambles_all").innerHTML = passcodes;
}