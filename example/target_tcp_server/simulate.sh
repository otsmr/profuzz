#!/bin/bash

echo "1" >reseted.txt

while true; do
  # Check if the file exists
  if [[ -f reseted.txt ]]; then
    # Read the content of the file
    content=$(<reseted.txt)

    # Check if the content is '1'
    if [[ "$content" == "1" ]]; then
      # Write '0' into the file
      echo "0" >reseted.txt
      echo "Restarting target"
      # Print 'OK'
      cargo run
    fi
  else
    echo "File reseted.txt does not exist."
  fi

  # Sleep for a short period to avoid busy waiting
  sleep 1
done
