import os
import shutil
from pathlib import Path

cwd = os.getcwd()
home = Path.home()

shutil.copytree(src=os.path.join(cwd, "queries", "lopa"),
                dst=os.path.join(home, ".config", "nvim", "queries", "lopa"),
                dirs_exist_ok=True)
