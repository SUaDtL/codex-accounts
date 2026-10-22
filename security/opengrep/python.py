import subprocess
import json
import pickle

# ruleid: python-shell-interpolation
subprocess.run(command, shell=True)
# ok: python-shell-interpolation
subprocess.run(["cargo", "test"], check=True)
# ruleid: python-untrusted-deserialization
pickle.loads(payload)
# ok: python-untrusted-deserialization
json.loads(payload)
