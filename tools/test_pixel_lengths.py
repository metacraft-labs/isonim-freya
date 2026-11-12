#!/usr/bin/env python3
"""Real public Nim FFI -> default owner shim -> original native parser witnesses.

The default shim supplies transport/plan inspection only, not a native window.
Native Size/Gaps/Radius parsers and the exact font numeric arm are separately
checksum-bound. Missing artifacts/sources/admission are hard failures, never skips.
"""
from pathlib import Path
import hashlib
import json
import os
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
PARENT = ROOT / 'build/pixel-lengths'
PARENT.mkdir(parents=True, exist_ok=True)
OUT = Path(tempfile.mkdtemp(prefix='run-', dir=PARENT))
ENV = dict(os.environ)
MINIMUM = int(ENV.get('ISONIM_TEST_MIN_AVAILABLE_BYTES', '0'))
# Cargo's target dir is not always rust/target (CARGO_TARGET_DIR, .cargo/config.toml);
# tools/cargo_target_dir.sh asks cargo, and a failure there is a hard failure.
TARGET_DIR = Path(subprocess.run([str(ROOT / 'tools/cargo_target_dir.sh')], check=True, stdout=subprocess.PIPE, text=True).stdout.strip())
LIBRARY = TARGET_DIR / 'debug' / ('libfreya_nim_shim.dylib' if os.uname().sysname == 'Darwin' else 'libfreya_nim_shim.so')
assert LIBRARY.is_file(), 'required default Rust shim missing; run just rust-build'
ENV['LD_LIBRARY_PATH'] = str(LIBRARY.parent) + os.pathsep + ENV.get('LD_LIBRARY_PATH', '')
print('retained ' + str(OUT), flush=True)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(name, argv, inputs=(), cwd=ROOT, env=ENV):
    tools = [Path(shutil.which(tool)) for tool in ['nim', 'rustc', 'cargo', 'python3', 'gcc', 'bash']]
    bound = {str(Path(path).resolve()): sha(path) for path in [*inputs, *tools, LIBRARY, Path(__file__), ROOT / 'rust/Cargo.lock', ROOT / 'rust/Cargo.toml',
        ROOT / 'flake.nix', ROOT / 'flake.lock', ROOT / 'nim.cfg', ROOT / 'config.nims']}
    stat = os.statvfs(OUT)
    disk = stat.f_bavail * stat.f_frsize
    memory = None
    if Path('/proc/meminfo').is_file():
        memory = next(int(line.split()[1]) * 1024 for line in Path('/proc/meminfo').read_text().splitlines() if line.startswith('MemAvailable:'))
    assert disk >= MINIMUM and (MINIMUM == 0 or memory is not None and memory >= MINIMUM), 'BLOCKED: disk/memory admission'
    receipt = dict(argv=list(map(str, argv)), cwd=str(cwd), inputs=bound, disk_available_bytes=disk, MemAvailable_bytes=memory, minimum_bytes=MINIMUM)
    (OUT / (name + '-BEFORE.json')).write_text(json.dumps(receipt, indent=2) + '\n')
    result = subprocess.run(list(map(str, argv)), cwd=cwd, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (OUT / (name + '.log')).write_text(result.stdout)
    receipt['exit'] = result.returncode
    receipt['inputs_after'] = {path: sha(path) for path in bound}
    assert receipt['inputs_after'] == bound, 'bound source/tool/artifact changed'
    (OUT / (name + '-RESULT.json')).write_text(json.dumps(receipt, indent=2) + '\n')
    print(name + ': ' + str(result.returncode), flush=True)
    if result.returncode:
        print(result.stdout, flush=True)
    return result


def block(text, marker):
    start = text.index(marker)
    depth = 0
    for index in range(text.index('{', start), len(text)):
        if text[index] == '{': depth += 1
        elif text[index] == '}':
            depth -= 1
            if depth == 0: return text[start:index + 1]
    raise AssertionError('original source block incomplete: ' + marker)


# Exact owner lock, not a floating downloaded implementation or rewritten parser.
lock = tomllib.loads((ROOT / 'rust/Cargo.lock').read_text())
sources = OUT / 'native-sources'
sources.mkdir()
packages = {}
for name in ['freya-core', 'torin']:
    package = next(row for row in lock['package'] if row['name'] == name)
    assert package['version'] == '0.3.4', 'native witness version requires review'
    archive = sources / (name + '-0.3.4.crate')
    url = 'https://static.crates.io/crates/' + name + '/' + archive.name
    archive.write_bytes(urllib.request.urlopen(url).read())
    assert sha(archive) == package['checksum'], 'native source checksum mismatch'
    with tarfile.open(archive) as contents:
        contents.extractall(sources, filter='data')
    packages[name] = dict(package, archive=str(archive), url=url)
(OUT / 'NATIVE-SOURCES.json').write_text(json.dumps(packages, indent=2) + '\n')
core = sources / 'freya-core-0.3.4/src'
parsing = (core / 'parsing.rs').read_text()
error = '#[derive(Clone, Debug, PartialEq)]\npub struct ParseError;'
assert error in parsing
trait = block(parsing, 'pub trait Parse: Sized')
radius = (core / 'values/corner_radius.rs').read_text()
structure = block(radius, 'pub struct CornerRadius')
fills = '\n'.join(block(radius, 'pub fn ' + name) for name in ['fill_top', 'fill_bottom', 'fill_all'])
radius_parse = block(radius, 'impl Parse for CornerRadius')
font = (core / 'states/font_style.rs').read_text()
font_arm = block(font, 'if let Ok(font_size) = value.parse::<f32>()')
assert 'self.font_size = font_size;' in font_arm
prefix = 'mod parsing {\n' + error + '\n' + trait + '\n}\n'
prefix += f'#[path="{core}/values/size.rs"] mod size_parser;\n#[path="{core}/values/gaps.rs"] mod gaps_parser;\n'
prefix += 'mod radius_parser {use crate::parsing::{Parse,ParseError};\n#[derive(PartialEq,Clone,Debug,Default,Copy)]\n' + structure + '\nimpl CornerRadius {\n' + fills + '\n}\n' + radius_parse + '\n}\n'
prefix += 'struct FontNumericArm {font_size:f32}\nimpl FontNumericArm {fn apply(&mut self,value:&str){\n' + font_arm + '\n}}\n'
probe = OUT / 'native-witness'
(probe / 'src').mkdir(parents=True)
witness = ROOT / 'tests/fixtures/pixel_native_witness.rs'
(probe / 'src/main.rs').write_text(prefix + witness.read_text())
native_manifest = tomllib.loads((sources / 'freya-core-0.3.4/Cargo.toml').read_text())
nom_major = native_manifest['dependencies']['nom']['version'].split('.')[0]
versions = {'nom': next(row['version'] for row in lock['package'] if row['name'] == 'nom' and row['version'].split('.')[0] == nom_major),
            'serde_json': next(row['version'] for row in lock['package'] if row['name'] == 'serde_json')}
(probe / 'Cargo.toml').write_text('[package]\nname="freya-pixel-native-witness"\nversion="0.0.0"\nedition="2021"\n[dependencies]\ntorin={path="' + str(sources / 'torin-0.3.4') + '"}\nnom="=' + versions['nom'] + '"\nserde_json="=' + versions['serde_json'] + '"\n')
shutil.copyfile(ROOT / 'rust/Cargo.lock', probe / 'Cargo.lock')
(OUT / 'EXTRACTION.json').write_text(json.dumps({name: hashlib.sha256(value.encode()).hexdigest() for name, value in dict(ParseError=error, Parse=trait, CornerRadius=structure + fills + radius_parse, FontNumericArm=font_arm).items()}, indent=2) + '\n')
parser_sources = [path for path in sources.rglob('*') if path.is_file()]
run('build-native-witness', ['cargo', 'build', '--manifest-path', probe / 'Cargo.toml'], [*parser_sources, witness, probe / 'src/main.rs', probe / 'Cargo.toml']).check_returncode()
# Honor the normal Cargo target directory if one is explicitly selected.
target = Path(ENV.get('CARGO_TARGET_DIR', str(probe / 'target')))
NATIVE = target / 'debug/freya-pixel-native-witness'
assert NATIVE.is_file(), 'native witness executable missing'
native_build_inputs = [probe / 'Cargo.lock', probe / 'Cargo.toml', probe / 'src/main.rs',
                       target / 'debug/freya-pixel-native-witness.d']
(OUT / 'NATIVE-BUILD.json').write_text(json.dumps({str(path): sha(path) for path in [NATIVE, *native_build_inputs]}, indent=2) + '\n')

# Actual normal compiler/lib/config, not a different ambient compiler.
dump = run('nim-dump', ['nim', 'dump', '--dump.format:json', '--hints:off', ROOT / 'tests/fixtures/pixel_length_capture.nim'])
dump.check_returncode()
info = json.loads(dump.stdout)
lib = Path(info['libpath'])
SOURCE_ROOT = Path(tempfile.mkdtemp(prefix='freya-pixel-source-'))
(OUT / 'SOURCE-ROOT.json').write_text(json.dumps({'path':str(SOURCE_ROOT)}) + '\n')
reference = (ROOT / 'src/isonim_freya/renderer.nim').read_text()
anchor = '    return scalarPixelValue(value)'
assert reference.count(anchor) == 1, 'production inverse anchor drift'
finite_anchor = 'classify(float32(parsed)) notin {fcNan, fcInf, fcNegInf}'
assert reference.count(finite_anchor) == 1, 'finite guard inverse anchor drift'
variants = [('base', reference), ('companion', reference),
            ('mutant', reference.replace(anchor, '    return value')),
            ('finite-companion', reference),
            ('finite-mutant', reference.replace(finite_anchor, 'abs(parsed) <= float(high(float32))'))]
nonowner_inputs = None
for name, body in variants:
    project = ROOT
    if name != 'base':
        project = SOURCE_ROOT / name / 'isonim-freya'
        shutil.copytree(ROOT / 'src', project / 'src')
        (project / 'src/isonim_freya/renderer.nim').write_text(body)
        for relative in ['nim.cfg', 'config.nims', 'tests/fixtures/pixel_length_capture.nim']:
            destination = project / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, destination)
        for sibling in ['isonim', 'nim-everywhere']:
            os.symlink(ROOT.parent / sibling, project.parent / sibling)
    fixture = project / 'tests/fixtures/pixel_length_capture.nim'
    selected = [path for folder in [project / 'src', lib] for path in folder.rglob('*') if path.is_file()]
    selected += [project / 'nim.cfg', project / 'config.nims', fixture]
    before = {str(path.resolve()): sha(path) for path in selected}
    (OUT / (name + '-SOURCE-BEFORE.json')).write_text(json.dumps(before, indent=2) + '\n')
    executable = OUT / (name + '-capture')
    cache = OUT / ('cache-' + name)
    cache.mkdir()
    run(name + '-compile', ['nim', 'c', '--genScript:on', '--nimcache:' + str(cache), '--out:' + str(executable), fixture], selected, cwd=project).check_returncode()
    after = {str(path.resolve()): sha(path) for path in selected}
    assert before == after
    (OUT / (name + '-SOURCE-AFTER.json')).write_text(json.dumps(after, indent=2) + '\n')
    deps = [Path(path) for path in (cache / 'pixel_length_capture.deps').read_text().splitlines() if path]
    assert deps and all(str(path.resolve()) in before for path in deps), 'unknown actual Nim input'
    assert (project / 'src/isonim_freya/renderer.nim').resolve() in [path.resolve() for path in deps], 'wrong actual production owner'
    actual_nonowner = {('lib/' + str(path.relative_to(lib)) if path.is_relative_to(lib) else
                        'project/' + str(path.relative_to(project))): sha(path)
                       for path in deps if path.resolve() != (project / 'src/isonim_freya/renderer.nim').resolve()}
    if nonowner_inputs is None:
        nonowner_inputs = actual_nonowner
    assert actual_nonowner == nonowner_inputs, 'actual nonowner compiler inputs differ across samepath companions/inverses'
    (OUT / (name + '-ACTUAL-INPUTS.json')).write_text(json.dumps({str(path.resolve()):sha(path) for path in deps}, indent=2) + '\n')
    # --genScript emits the compiler's own C build script and exact .deps.
    # Execute that unmodified generated build, so native execution and the
    # dependency record describe the SAME Nim-to-C translation.
    scripts = list(cache.glob('compile_*.sh'))
    assert len(scripts) == 1, 'normal Nim generated build script missing'
    generated = [path for path in cache.iterdir() if path.is_file() and path.suffix in ['.c', '.h', '.sh']]
    run(name + '-native-build', ['bash', scripts[0]], generated, cwd=cache).check_returncode()
    executable = cache / executable.name
    assert executable.is_file(), 'native capture executable missing after generated build'
    captured = OUT / (name + '-capture.json')
    run(name + '-public-ffi', [executable, captured], [executable, LIBRARY, fixture]).check_returncode()
    rows = json.loads(captured.read_text())['rows']
    mismatches = [row['name'] for row in rows if row['actual'] != row['expected'] or row['present'] != row['expectedPresent']]
    (OUT / (name + '-PLAN-ASSERTIONS.json')).write_text(json.dumps({'rows':len(rows),'mismatches':mismatches}, indent=2) + '\n')
    # Native calls ALWAYS run on the captured actual values, even if the exact
    # plan check above disagrees. A plan-only kill cannot claim native coverage.
    native_output = OUT / (name + '-NATIVE-ASSERTIONS.json')
    native = run(name + '-native', [NATIVE, captured, native_output], [NATIVE, captured, witness, *parser_sources, *native_build_inputs])
    evidence = json.loads(native_output.read_text())
    if name == 'mutant':
        assert mismatches and native.returncode == 101 and evidence['failures'], 'genuine native production inverse not discriminating'
        positive = {row['name'] for row in rows if row['native'] == 'finite' and row['pair']}
        assert any(row['name'] in positive and row.get('payload', '').endswith('px') and row.get('accepted') is False and
                   'native-compatibility:' + row['name'] in evidence['failures'] for row in evidence['outputs']), 'actual positive px native refusal missing'
    elif name == 'finite-mutant':
        assert 'preserved-1e39px' in mismatches and native.returncode == 101, 'finite guard production inverse not discriminating'
        assert any(row['name'] == 'preserved-1e39px' and row['payload'] == '1e39' and
                   row['accepted'] and not row['finite'] and
                   'native-compatibility:preserved-1e39px' in evidence['failures']
                   for row in evidence['outputs']), 'actual overflow payload/native nonfinite refusal missing'
    else:
        assert not mismatches, 'semantic: exact-public-plan-payload: ' + str(mismatches)
        native.check_returncode()
print('PASS real public FFI, original native parsers, preservation and samepath production inverse', flush=True)
