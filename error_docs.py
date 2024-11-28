import re

ERROR_INITIAL_DOC = "/// Macro to produce nice errors"
ERRORS_PATH = "./src/errors.rs"
ERROR_PATTERN = r'"(E\d\d\d\d:.+?)"'
ERROR_DOC_LISTING_PATTERN = rf"({ERROR_INITIAL_DOC}).*?(#\[macro_export\])"
with open(ERRORS_PATH, "r") as fr:
    content = fr.read()
    errors = re.findall(ERROR_PATTERN, content)
    new_content = re.sub(
        ERROR_DOC_LISTING_PATTERN,
        r"\g<1>\n/// " + "\n/// ".join(errors) + r"\n\g<2>",
        content,
        0,
        re.DOTALL,
    )
    with open(ERRORS_PATH, "w") as fw:
        print(new_content)
        fw.write(new_content)
