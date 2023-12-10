# Rust WASM Scramble Sorter

A tool for sorting your scrambles and individual passcodes for WCA competitions! 

The scenario where you would use this tool is when you are using an electronic device for displaying the scrambles at a competition.

You will get a txt file with all the passcodes in the order that they will be used during the competition,  and a zip with numbers prepended such that the scrambles will be in order

The program will run all the required code in your browser. Thus, the scramble file is not sent to a server or stored anywhere.  Your internet connection is only used to send an API call to the WCA website such that the program will know the schedule and can do the sorting accordingly.

### Build locally
Run the command `wasm-pack build --target web`