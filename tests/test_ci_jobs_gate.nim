## Every CI job and step in this repository gates its workflow run.
##
## WHY.  GitHub Actions' `continue-on-error: true` (on a job or on a step)
## lets that job or step fail while the workflow run still concludes
## `success`, so a pull request's check rollup stays green over a failing
## test.  See
## https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions#jobsjob_idcontinue-on-error
## and `#jobsjob_idstepscontinue-on-error`.  The `gui-tests` job carried it
## from its creation until the real-Freya-backend hang it masked was fixed;
## while it was there, that hang blocked no merge for seven months.  A test
## that is run but whose failure is ignored is a skipped test, so the setting
## is refused outright: if a job is too unreliable to gate, the remedy is to
## fix it, not to un-gate it.
##
## WHAT THIS CHECKS.
##   1. CONTROL.  The scanner flags each form the key can take (a literal
##      `true`, an expression, a quoted value, a flow-mapping entry) in
##      fixture text, and accepts a literal `false`, comments and the key's
##      name inside a `run:` string.  Without this the scan below could pass
##      by matching nothing.
##   2. WORKFLOWS.  Every `.github/workflows/*.yml` / `*.yaml` file has no
##      `continue-on-error` whose value is anything but `false`, and there is
##      at least one workflow file to scan.
##
## The scan is textual rather than a YAML parse: this repository has no YAML
## library, and a key named `continue-on-error` cannot appear in a workflow
## file other than as that key (or in a comment or a string, which the
## scanner handles).  It errs toward flagging.
##
## NO MOCKS: the fixtures in step 1 are inline text fed to the same function
## that step 2 applies to the real files.

import std/[os, strutils]

const
  RepoRoot = currentSourcePath().parentDir.parentDir
  Key = "continue-on-error"

proc stripComment(line: string): string =
  ## Drops a YAML comment: a `#` at the start of the line or after
  ## whitespace, outside quotes.  Good enough for workflow files, where a
  ## `#` inside a value is always inside quotes or inside `${{ }}`.
  var inSingle, inDouble = false
  for i, c in line:
    case c
    of '\'':
      if not inDouble: inSingle = not inSingle
    of '"':
      if not inSingle: inDouble = not inDouble
    of '#':
      if not inSingle and not inDouble and (i == 0 or line[i-1] in Whitespace):
        return line[0 ..< i]
    else: discard
  line

proc offendingLines*(text: string): seq[(int, string)] =
  ## Returns (1-based line number, line) for each `continue-on-error` key
  ## whose value is not a literal `false`.
  var lineNo = 0
  for raw in text.splitLines:
    inc lineNo
    let line = stripComment(raw)
    var start = 0
    while true:
      let at = line.find(Key, start)
      if at < 0: break
      start = at + Key.len
      # A key, not a substring of some other word or of a `run:` command:
      # preceded by indentation, `- `, `{` or `,`, and followed by `:`.
      var before = line[0 ..< at].strip(leading = false)
      if before.endsWith("\"") or before.endsWith("'"):   # a quoted key
        before = before[0 ..< ^1].strip(leading = false)
      let isKeyPos = before.len == 0 or before.strip.len == 0 or
        before.endsWith("-") or before.endsWith("{") or before.endsWith(",")
      var rest = line[start .. ^1]
      for q in ["\"", "'"]:
        if rest.startsWith(q): rest = rest[1 .. ^1]   # a quoted key
      # YAML allows spaces between a key and its `:` (`key : value`).
      rest = rest.strip(trailing = false)
      if not isKeyPos or not rest.startsWith(":"): continue
      var value = rest[1 .. ^1].strip
      # Within a flow mapping the value ends at `,` or `}`.
      for sep in [',', '}']:
        let cut = value.find(sep)
        if cut >= 0 and not value.startsWith("${{"): value = value[0 ..< cut].strip
      value = value.strip(chars = {'"', '\''} + Whitespace)
      if value != "false":
        result.add((lineNo, raw))

var failures = 0

proc check(cond: bool, msg: string) =
  if cond:
    echo "  ok    ", msg
  else:
    inc failures
    echo "  FAIL  ", msg

# --- 1. CONTROL --------------------------------------------------------------
block:
  let flagged = [
    "  gui-tests:\n    runs-on: x\n    continue-on-error: true\n",
    "    continue-on-error: ${{ matrix.experimental }}\n",
    "      - name: x\n        continue-on-error: \"true\"\n",
    "      - continue-on-error: True\n        run: x\n",
    "    strategy: { fail-fast: false, continue-on-error: true }\n",
    "    \"continue-on-error\": yes\n",
    "    continue-on-error:   true   # flaky runner\n",
    "    continue-on-error : true\n",
    "    \"continue-on-error\" : true\n",
    "    continue-on-error:\n      true\n",
  ]
  for t in flagged:
    check offendingLines(t).len == 1, "scanner flags " & t.strip.escape
  let accepted = [
    "    continue-on-error: false\n",
    "    # continue-on-error: true was removed\n",
    "        run: echo \"no continue-on-error: true here\"\n",
    "    name: continue-on-error-free\n",
  ]
  for t in accepted:
    check offendingLines(t).len == 0, "scanner accepts " & t.strip.escape

# --- 2. WORKFLOWS ------------------------------------------------------------
block:
  let dir = RepoRoot / ".github" / "workflows"
  var scanned = 0
  for kind, path in walkDir(dir):
    if kind notin {pcFile, pcLinkToFile}: continue
    if path.splitFile.ext notin [".yml", ".yaml"]: continue
    inc scanned
    let rel = relativePath(path, RepoRoot)
    let bad = offendingLines(readFile(path))
    for (n, line) in bad:
      echo "        ", rel, ":", n, ": ", line.strip
    check bad.len == 0,
      rel & " has no `continue-on-error` that lets a failure pass"
  check scanned > 0, "found workflow files to scan under " & dir

if failures > 0:
  echo failures, " check(s) failed"
  quit 1
echo "all checks passed"
