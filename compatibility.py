from posixpath import splitext
import xmltodict
import inflection as i
import toml
from os import path as p


def numerical(string: str):
    """
    Tries to convert string to numerical.
    A better implementation could be found,
    but this works and it's for the shake of retrocompatibility
    """

    if string == "true":
        return True

    elif string == "false":
        return False

    else:
        try:
            return int(string)
        
        except:
            try:
                return float(string)
            except:
                return string


def config_converter(path: str) -> None:

    # Parse XML file
    with open(path) as file:
        doc = xmltodict.parse(
            file.read(), process_namespaces=True, attr_prefix='')

        data = {i.underscore(x): {i.underscore(
            y): numerical(doc['config'][x][y]) for y in doc['config'][x]} for x in doc['config']}
    
    # Write TOML file
    with open(p.splitext(path)[0] + '.toml', 'w') as file:
        toml.dump(data, file)


if __name__ == '__main__':

    config_converter('config.xml')
