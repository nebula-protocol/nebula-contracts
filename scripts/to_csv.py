import csv
import json
import os
from datetime import date
from dotenv import load_dotenv

load_dotenv()


def main():
    today_date = date.today().strftime("%d%m%Y")
    file = open(f"../artifacts/{os.environ['CHAIN_ID']}.json")
    info = json.load(file)
    addresses_dict = {}
    for (key, value) in info.items():
        if "Address" in key:
            addresses_dict[key] = value
    with open(f"deployments/neb_deployment_{today_date}", "w") as csv_file:
        for key in addresses_dict.keys():
            csv_file.write("%s, %s\n" % (key, addresses_dict[key]))


if __name__ == "__main__":
    main()
