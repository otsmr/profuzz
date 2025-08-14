import os

os.makedirs("../corpus", exist_ok=True)

with open("trace.txt", "r") as file:
    trace_data = file.read()

messages = trace_data.split("Client -> Server")

# Initialize a counter for naming the binary files
file_counter = 0

# Iterate through the messages to find Client -> Server messages
for message in messages:
    c2s = message.split("Server -> Client")[0].strip()
    hexlines = c2s.split("\n")

    hex = ""
    for line in hexlines:
        hexs = line.split("  ")
        if len(hexs) > 1:
            hex += hexs[1]
        if len(hexs) > 2:
            hex += hexs[2]
    hex = hex.replace(" ", "")
    if hex == "":
        continue
    print(hex)
    binary_data = bytes.fromhex(hex)

    # Save the binary data to a file
    file_name = f"../corpus/message_{file_counter:02}.bin"
    with open(file_name, "wb") as bin_file:
        bin_file.write(binary_data)

    # Increment the file counter
    file_counter += 1

print(
    f"Successfully saved {file_counter} Client -> Server messages to the corpus directory."
)
