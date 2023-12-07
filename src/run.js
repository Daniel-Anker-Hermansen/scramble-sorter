import init, {run} from "./clock_app.js";


let elem = document.getElementById('submit_button');
elem.addEventListener('click',run_sorting);

const wasm = init();

export function run_sorting(){
    let memo = wasm.then(()=>run()).then(update_elements);
}

function update_elements(passcodes){
    console.log(passcodes);
    document.getElementById("scrambles_all").innerHTML = passcodes;
}