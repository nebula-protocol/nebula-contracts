#!/bin/bash

cd common_docker_files

for folder in `ls ../clusters`
do 
    for file in *.py
    do
       cp "$file" "../clusters/${folder}/src/${file}"
    done

    for artifact in `ls artifacts`
    do
        cp "artifacts/$artifact" "../clusters/${folder}/artifacts/${artifact}"
    done
done
