import re
with open('crates/dbg-core/src/internal_runtime.rs', 'r') as f:
    content = f.read()

# remove duplicate lines
lines = content.split('\n')
out = []
seen_func = False
seen_call = False
for line in lines:
    if 'function_id: None,' in line:
        if seen_func: continue
        seen_func = True
    if 'call_id: None,' in line:
        if seen_call: continue
        seen_call = True
    if 'Node {' in line:
        seen_func = False
        seen_call = False
    
    out.append(line)

with open('crates/dbg-core/src/internal_runtime.rs', 'w') as f:
    f.write('\n'.join(out))
