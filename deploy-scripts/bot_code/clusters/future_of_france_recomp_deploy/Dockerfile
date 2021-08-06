FROM python:3.8

# set the working directory in the container
WORKDIR /code

# copy the dependencies file to the working directory
COPY requirements.txt .

# install dependencies
RUN pip install -r requirements.txt

# copy the content of the local src directory to the working directory
COPY src/ src/

# copy wasm artifacts to the working directory
COPY artifacts/ artifacts/

CMD [ "python", "-u", "src/recomp_deploy.py"]