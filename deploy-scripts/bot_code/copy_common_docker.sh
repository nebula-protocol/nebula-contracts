#!/bin/bash

cd common_docker_files

for folder in `ls ../clusters`
do 
    for file in *.py
    do
        cp -v "$file" "clusters/${folder}/src/${file}"
    done
done
