# Usage: cargo build && python3 scripts/check_compile_fail.py .
# Checks every `compile_fail,EXXXX` doctest in src/: it must fail with exactly that error code,
# and must compile once the line marked `// error` is removed (not applicable when that line is
# the whole offending item, e.g. the E0053 snippet). When rustc reports a second code for the
# same mistake, the `// error` line says so with "also reports EYYYY", and both codes must appear.
# The library is taken from the target directory that `cargo build` used: `cargo metadata`
# resolves it (CARGO_TARGET_DIR, `build.target-dir`, or `<root>/target`).
import re, subprocess, sys, pathlib, json
root = pathlib.Path(sys.argv[1]).resolve()
meta = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=root,
                      capture_output=True, text=True, check=True)
target = pathlib.Path(json.loads(meta.stdout)["target_directory"])
lib = target / "debug/libstatechart.rlib"
deps = target / "debug/deps"
if not lib.exists():
    sys.exit(f"{lib} does not exist: run `cargo build` first")
out = target / "compile_fail_check"; out.mkdir(parents=True, exist_ok=True)
n = 0; ok = True
for f in sorted((root/"src").glob("*.rs")):
    lines = f.read_text().splitlines()
    i = 0
    while i < len(lines):
        m = re.match(r'\s*(///|//!) ```compile_fail,(E\d+)', lines[i])
        if m:
            code = m.group(2); body = []; i += 1
            while not re.match(r'\s*(///|//!) ```\s*$', lines[i]):
                l = re.sub(r'^\s*(///|//!) ?', '', lines[i]); l = re.sub(r'^# ?', '', l) if l.startswith('#') and not l.startswith('#[') and not l.startswith('#!') else l
                body.append(l); i += 1
            attrs = [l for l in body if l.startswith('#!')]; rest = [l for l in body if not l.startswith('#!')]
            def build(lines_, tag):
                src = "\n".join(attrs + ["#![allow(dead_code, unused)]", "use statechart as _;", "fn main() {"] + lines_ + ["}"])
                p = out / f"cf{n}_{tag}.rs"; p.write_text(src)
                r = subprocess.run(["rustc","--edition","2024","--crate-type","bin","-L",str(deps),"--extern",f"statechart={lib}","-o",str(out/f"cf{n}_{tag}"),str(p)],capture_output=True,text=True)
                return r
            bad = build(rest, "bad")
            err_lines = [l for l in rest if "// error" in l]
            whole_item = any(re.match(r'\s*fn ', l) for l in err_lines)
            good = build([l for l in rest if "// error" not in l], "good")
            errs = sorted(set(re.findall(r'error\[(E\d+)\]', bad.stderr)))
            expected = sorted({code, *re.findall(r'also reports (E\d+)', "\n".join(err_lines))})
            good_ok = good.returncode == 0 or whole_item
            status = "OK" if (bad.returncode != 0 and errs == expected and good_ok) else "MISMATCH"
            if status != "OK": ok = False
            good_desc = "n/a (error line is a whole fn item)" if whole_item else (good.returncode == 0)
            print(f"{f.name}:{i} expect {expected} got {errs} compiles_without_error_line={good_desc} -> {status}")
            if not good_ok: print(good.stderr[:2000])
            n += 1
        i += 1
sys.exit(0 if ok else 1)
