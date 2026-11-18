# T-074 probe, round 1 (planner probe on `ci/candidate`)

## Run
- Workflow: `platforms.yml`, run id 44290286460, run number 25, event `push`, branch `ci/candidate`.
- Head sha: `de8106fe621e26ec907b6e1a2a3c068438888992` (commit "T-074 planner probe (temporary, not for merge): Windows reparse points, held root, names").
- Created 2026-11-18T10:57:57Z. Status when read: `in_progress` (the gate jobs below were still running).

## Jobs (as read from `list_workflow_jobs`)
| job | id | conclusion | first failing step |
|---|---|---|---|
| `probe-t074` (windows-2025) | 134753444705 | success | none (step "T-074 probe (reported)" 10:58:16Z to 10:58:31Z) |
| `window-linux` | 134753444426 | success | none |
| `window-probe (macos-15)` | 134753445105 | success | none |
| `window-probe (windows-2025)` | 134753444820 | failure | "Window probe" |
| `gate-windows` | 134753444909 | in progress when last read (step "Gate" failure, 11:04:19Z; later report steps running) | "Gate" |
| `gate-macos` | 134753444878 | in progress when last read ("Gate" step still running) | not read |
| `gate-linux` | 134753444903 | in progress when last read (step "Gate" failure, 11:06:42Z; later report steps running) | "Gate" |

The gate jobs were not complete when this record was written; their final conclusions and the failing gate
step inside each (the brief expects `no-skip`, for the unregistered probe file) are not read here.

## How the output was obtained
- Artefact `probe-t074-44290286460` (id 14419864493, 3785 bytes zipped, created 2026-11-18T10:58:32Z), listed by
  `list_workflow_run_artifacts`, `download_url` from `download_workflow_run_artifact`, fetched with curl; zip size
  equalled `size_in_bytes`; `testzip` clean; unpacked file `probe-t074.txt` is 23218 bytes, 274 lines.
- Below: every line of `probe-t074.txt` that begins with `PROBE`, `test `, `thread` or `error`, after stripping
  ANSI colour escapes (`ESC[...m`), in file order, otherwise unedited, grouped under the test each belongs to. 195
  lines. No line begins with `thread` or `error`. Lines not shown: the user name line, cargo `Updating`,
  `Downloaded`, `Compiling`, `Finished`, `Running` lines, blank lines, the per-test `ok` lines and
  `running 7 tests`. Note: each `test win::<name> ... ` line carries its first `PROBE` line on the same line (cargo
  prints the test name, then the test's own stdout). One `PROBE I9` line ends in `…` in the artefact itself.
- Complete: yes for the artefact (the run ended with `PROBE T-074: exit status 0`, `7 passed; 0 failed`).

## Probe output

### Header

```text
PROBE OS: MINGW64_NT-10.0-26100 runnervmm7ud1 3.6.10-5a1665c8.x86_64 2026-09-28 10:40 UTC x86_64 Msys
```

### Test `win::a_b_links_and_attributes`

```text
test win::a_b_links_and_attributes ... PROBE A1: symlink_file: Ok(())
PROBE A2: symlink_dir: Ok(())
PROBE A3: mklink /J inside: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\root\\j <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\root\\d" stderr ""
PROBE A4: mklink /J outside: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\root\\jo <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\out" stderr ""
PROBE A5: symlink_file outside: Ok(())
PROBE A6: symlink_file relative inside: Ok(())
PROBE B1: symlink_metadata f.txt: is_symlink=false is_dir=false is_file=true attrs=0x2020 reparse=false
PROBE B2: read_link f.txt: Err(Os { code: 4390, kind: Uncategorized, message: "The file or directory is not a reparse point." })
PROBE B1: symlink_metadata d: is_symlink=false is_dir=true is_file=false attrs=0x2010 reparse=false
PROBE B2: read_link d: Err(Os { code: 4390, kind: Uncategorized, message: "The file or directory is not a reparse point." })
PROBE B1: symlink_metadata sf: is_symlink=true is_dir=false is_file=false attrs=0x2420 reparse=true
PROBE B2: read_link sf: Ok("D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\root\\f.txt")
PROBE B1: symlink_metadata sd: is_symlink=true is_dir=false is_file=false attrs=0x2410 reparse=true
PROBE B2: read_link sd: Ok("D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\root\\d")
PROBE B1: symlink_metadata j: is_symlink=true is_dir=false is_file=false attrs=0x2410 reparse=true
PROBE B2: read_link j: Ok("D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\root\\d")
PROBE B1: symlink_metadata jo: is_symlink=true is_dir=false is_file=false attrs=0x2410 reparse=true
PROBE B2: read_link jo: Ok("D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\out")
PROBE B1: symlink_metadata so: is_symlink=true is_dir=false is_file=false attrs=0x2420 reparse=true
PROBE B2: read_link so: Ok("D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\out\\o.txt")
PROBE B1: symlink_metadata sr: is_symlink=true is_dir=false is_file=false attrs=0x2420 reparse=true
PROBE B2: read_link sr: Ok("f.txt")
PROBE B3: read_dir root: "d"[ft(sym,dir)=Ok((false, true)) attrs=Ok(8208)], "f.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "j"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9232)], "jo"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9232)], "sd"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9232)], "sf"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9248)], "so"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9248)], "sr"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9248)]
PROBE B4: canonicalize root: Ok("\\\\?\\D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\ab\\root")
PROBE B5: temp dir attrs: is_symlink=false is_dir=true is_file=false attrs=0x10 reparse=false
PROBE B6: CARGO_TARGET_TMPDIR canonical: Ok("\\\\?\\D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp")
PROBE C1: read f.txt: Ok([102])
PROBE C2: check_location f.txt: Ok(())
PROBE C1: read sf: Err(OutsideRoot { path: RelPath("sf") })
PROBE C2: check_location sf: Err(OutsideRoot { path: RelPath("sf") })
PROBE C1: read sd/x: Err(OutsideRoot { path: RelPath("sd/x") })
PROBE C2: check_location sd/x: Err(OutsideRoot { path: RelPath("sd/x") })
PROBE C1: read j: Err(OutsideRoot { path: RelPath("j") })
PROBE C2: check_location j: Err(OutsideRoot { path: RelPath("j") })
PROBE C1: read j/x: Err(OutsideRoot { path: RelPath("j/x") })
PROBE C2: check_location j/x: Err(OutsideRoot { path: RelPath("j/x") })
PROBE C1: read jo/o.txt: Err(OutsideRoot { path: RelPath("jo/o.txt") })
PROBE C2: check_location jo/o.txt: Err(OutsideRoot { path: RelPath("jo/o.txt") })
PROBE C1: read so: Err(OutsideRoot { path: RelPath("so") })
PROBE C2: check_location so: Err(OutsideRoot { path: RelPath("so") })
PROBE C1: read sr: Ok([102])
PROBE C2: check_location sr: Ok(())
PROBE C3: read j/x (file exists in d): Err(OutsideRoot { path: RelPath("j/x") })
PROBE C3: read sd/x (file exists in d): Err(OutsideRoot { path: RelPath("sd/x") })
PROBE C4: write_atomic sf: Err(OutsideRoot { path: RelPath("sf") })
PROBE C4: after: f.txt: Ok([102])
PROBE C4: after: sf attrs: is_symlink=true is_dir=false is_file=false attrs=0x2420 reparse=true
PROBE C5: write_atomic jo/o.txt: Err(OutsideRoot { path: RelPath("jo/o.txt") })
PROBE C5: after: out/o.txt: Ok([111])
PROBE C6: write_atomic j/n.txt: Err(OutsideRoot { path: RelPath("j/n.txt") })
PROBE C6: after: d listing: "x"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)]
PROBE C7: list_dir root: Ok(["d:Dir", "f.txt:File", "j:Symlink", "jo:Symlink", "sd:Symlink", "sf:Symlink", "so:Symlink", "sr:Symlink"])
PROBE C8: list_dir j: Err(OutsideRoot { path: RelPath("j") })
PROBE C9: listing root after: "d"[ft(sym,dir)=Ok((false, true)) attrs=Ok(8208)], "f.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "j"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9232)], "jo"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9232)], "sd"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9232)], "sf"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9248)], "so"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9248)], "sr"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9248)]
```

### Test `win::e_g_root_through_and_replaced_by_junction`

```text
test win::e_g_root_through_and_replaced_by_junction ... PROBE E1: mklink /J link->real: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\link <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\real" stderr ""
PROBE E2: DiskFs::new(link): Ok(())
PROBE E3: read f.txt via link root: Ok("real")
PROBE E4: remove_dir(link junction): Ok(())
PROBE E4: mklink /J link->other: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\link <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\other" stderr ""
PROBE E5: read f.txt after retarget: Err(OutsideRoot { path: RelPath("f.txt") })
PROBE E5: other/f.txt: Ok([111, 116, 104, 101, 114])
PROBE E6: remove_dir(link) again: Ok(())
PROBE E6: mklink /J link->real: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\link <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\real" stderr ""
PROBE E7: read f.txt after link back to real: Ok(4)
PROBE G1: rename root -> root.real: Ok(())
PROBE G2: mklink /J root->other: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\root <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\other" stderr ""
PROBE G3: read f.txt: Err(OutsideRoot { path: RelPath("f.txt") })
PROBE G3: write_atomic f.txt: Err(OutsideRoot { path: RelPath("f.txt") })
PROBE G3: other/f.txt: Ok([111, 116, 104, 101, 114])
PROBE G4: remove junction root: Ok(())
PROBE G4: mklink /J root->root.real: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\root <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\eg\\root.real" stderr ""
PROBE G5: read f.txt via junction to held root: Ok(4)
```

### Test `win::f_held_root_removed`

```text
test win::f_held_root_removed ... PROBE F1: remove_dir(held empty root): Ok(())
PROBE F2: root exists after: false symlink_metadata Err(Os { code: 2, kind: NotFound, message: "The system cannot find the file specified." })
PROBE F3: check_location x: Err(OutsideRoot { path: RelPath("x") })
PROBE F4: create_dir root again: Ok(())
PROBE F5: check_location x after new root: Err(OutsideRoot { path: RelPath("x") })
PROBE F5: list_dir after new root: Err(OutsideRoot { path: RelPath("(root)") })
PROBE F6: root exists after drop: true
PROBE F7: remove_dir_all(held root with file): Ok(())
PROBE F7: root2 exists after: false
PROBE F8: read f.txt: Err(OutsideRoot { path: RelPath("f.txt") })
```

### Test `win::h_link_at_temp_name`

```text
test win::h_link_at_temp_name ... PROBE H1: mklink /J .kiln-tmp-0 -> out: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\h\\root\\.kiln-tmp-0 <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\h\\out" stderr ""
PROBE H1: symlink_file .kiln-tmp-1 -> out/t: Ok(())
PROBE H2: write_atomic f.txt: Err(Io { path: RelPath("f.txt"), kind: PermissionDenied, message: "Access is denied. (os error 5)" })
PROBE H3: f.txt: Ok([111, 108, 100])
PROBE H3: root listing: ".kiln-tmp-0"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9232)], ".kiln-tmp-1"[ft(sym,dir)=Ok((true, false)) attrs=Ok(9248)], "f.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)]
PROBE H3: out listing: 
```

### Test `win::i_names_and_case`

```text
test win::i_names_and_case ... PROBE I1: write_atomic "CON": Ok(())
PROBE I2: read "CON": Ok([119])
PROBE I3: check_location "CON": Ok(())
PROBE I1: write_atomic "con.txt": Ok(())
PROBE I2: read "con.txt": Ok([119])
PROBE I3: check_location "con.txt": Ok(())
PROBE I1: write_atomic "NUL": Ok(())
PROBE I2: read "NUL": Ok([119])
PROBE I3: check_location "NUL": Ok(())
PROBE I1: write_atomic "nul.rs": Ok(())
PROBE I2: read "nul.rs": Ok([119])
PROBE I3: check_location "nul.rs": Ok(())
PROBE I1: write_atomic "COM1": Ok(())
PROBE I2: read "COM1": Ok([119])
PROBE I3: check_location "COM1": Ok(())
PROBE I1: write_atomic "LPT1.txt": Ok(())
PROBE I2: read "LPT1.txt": Ok([119])
PROBE I3: check_location "LPT1.txt": Ok(())
PROBE I1: write_atomic "AUX": Ok(())
PROBE I2: read "AUX": Ok([119])
PROBE I3: check_location "AUX": Ok(())
PROBE I1: write_atomic "PRN": Ok(())
PROBE I2: read "PRN": Ok([119])
PROBE I3: check_location "PRN": Ok(())
PROBE I1: write_atomic "CONIN$": Ok(())
PROBE I2: read "CONIN$": Ok([119])
PROBE I3: check_location "CONIN$": Ok(())
PROBE I1: write_atomic "COM¹": Ok(())
PROBE I2: read "COM¹": Ok([119])
PROBE I3: check_location "COM¹": Ok(())
PROBE I1: write_atomic "a.": Ok(())
PROBE I2: read "a.": Ok([119])
PROBE I3: check_location "a.": Ok(())
PROBE I1: write_atomic "b ": Ok(())
PROBE I2: read "b ": Ok([119])
PROBE I3: check_location "b ": Ok(())
PROBE I1: write_atomic "c. .": Ok(())
PROBE I2: read "c. .": Ok([119])
PROBE I3: check_location "c. .": Ok(())
PROBE I1: write_atomic "f.txt:s": Err(Io { path: RelPath("f.txt:s"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "f.txt:s": Err(NotFound { path: RelPath("f.txt:s") })
PROBE I3: check_location "f.txt:s": Ok(())
PROBE I1: write_atomic "f.txt:s:$DATA": Err(Io { path: RelPath("f.txt:s:$DATA"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "f.txt:s:$DATA": Err(NotFound { path: RelPath("f.txt:s:$DATA") })
PROBE I3: check_location "f.txt:s:$DATA": Ok(())
PROBE I1: write_atomic "f.txt::$DATA": Err(Io { path: RelPath("f.txt::$DATA"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "f.txt::$DATA": Ok([102])
PROBE I3: check_location "f.txt::$DATA": Ok(())
PROBE I1: write_atomic "C:x": Err(OutsideRoot { path: RelPath("C:x") })
PROBE I2: read "C:x": Err(OutsideRoot { path: RelPath("C:x") })
PROBE I3: check_location "C:x": Err(OutsideRoot { path: RelPath("C:x") })
PROBE I1: write_atomic "C:": Err(OutsideRoot { path: RelPath("C:") })
PROBE I2: read "C:": Err(OutsideRoot { path: RelPath("C:") })
PROBE I3: check_location "C:": Err(OutsideRoot { path: RelPath("C:") })
PROBE I1: write_atomic "D:x": Err(OutsideRoot { path: RelPath("D:x") })
PROBE I2: read "D:x": Err(OutsideRoot { path: RelPath("D:x") })
PROBE I3: check_location "D:x": Err(OutsideRoot { path: RelPath("D:x") })
PROBE I1: write_atomic "x<y": Err(Io { path: RelPath("x<y"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "x<y": Err(Io { path: RelPath("x<y"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I3: check_location "x<y": Err(Io { path: RelPath("x<y"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I1: write_atomic "q?": Err(Io { path: RelPath("q?"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "q?": Err(Io { path: RelPath("q?"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I3: check_location "q?": Err(Io { path: RelPath("q?"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I1: write_atomic "s*": Err(Io { path: RelPath("s*"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "s*": Err(Io { path: RelPath("s*"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I3: check_location "s*": Err(Io { path: RelPath("s*"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I1: write_atomic "p|q": Err(Io { path: RelPath("p|q"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "p|q": Err(Io { path: RelPath("p|q"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I3: check_location "p|q": Err(Io { path: RelPath("p|q"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I1: write_atomic "dq\"": Err(Io { path: RelPath("dq\""), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "dq\"": Err(Io { path: RelPath("dq\""), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I3: check_location "dq\"": Err(Io { path: RelPath("dq\""), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I1: write_atomic "ctl\u{1}": Err(Io { path: RelPath("ctl\u{1}"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I2: read "ctl\u{1}": Err(Io { path: RelPath("ctl\u{1}"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I3: check_location "ctl\u{1}": Err(Io { path: RelPath("ctl\u{1}"), kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is incorrect. (os error 123)" })
PROBE I1: write_atomic "x/C:y": Err(NotFound { path: RelPath("x/C:y") })
PROBE I2: read "x/C:y": Err(NotFound { path: RelPath("x/C:y") })
PROBE I3: check_location "x/C:y": Ok(())
PROBE I4: f.txt after: Ok([102])
PROBE I5: root listing after: "AUX"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "COM1"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "COM¹"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "CON"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "CONIN$"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "LPT1.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "NUL"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "PRN"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "a."[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "averylongdirectoryname"[ft(sym,dir)=Ok((false, true)) attrs=Ok(8208)], "b "[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "c. ."[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "con.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "f.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "nul.rs"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)]
PROBE I6: list_dir root: Ok((["AUX", "COM1", "COM¹", "CON", "CONIN$", "LPT1.txt", "NUL", "PRN", "a.", "averylongdirectoryname", "b ", "c. .", "con.txt", "f.txt", "nul.rs"], 0))
PROBE I7: read A.TXT? (case) F.TXT: Ok([102])
PROBE I7: check_location F.TXT: Ok(())
PROBE I8: read short name AVERYL~1/g.txt: Err(NotFound { path: RelPath("AVERYL~1/g.txt") })
PROBE I9: fsutil 8dot3name query C:: Ok("The volume state is: 0 (8dot3 name creation is ENABLED)  The registry state is: 2 (Per volume setting - the default)    Based on the above settings, 8dot3 n…
PROBE I9: cwd: "D:\\a\\photocraft-study-review\\photocraft-study-review\\crates\\kiln-workspace"
PROBE I9: fsutil 8dot3name query <cwd volume>: Ok("The volume state is: 1 (8dot3 name creation is DISABLED)  The registry state is: 2 (Per volume setting - the default)    Based on the above settings, 8dot3 …
PROBE I9: fsutil fsinfo volumeinfo <cwd volume>: Ok("Error 3: The system cannot find the path specified.  ")
PROBE I10: canonical.join("C:x"): "C:x"
PROBE I10: canonical.join("CON"): "\\\\?\\D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\i\\root\\CON"
PROBE I11: fs::write(non-verbatim root/CON): Ok(())
PROBE I11: fs::write(non-verbatim root/a.): Ok(())
PROBE I12: root listing at end: "AUX"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "COM1"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "COM¹"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "CON"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "CONIN$"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "LPT1.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "NUL"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "PRN"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "a"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "a."[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "averylongdirectoryname"[ft(sym,dir)=Ok((false, true)) attrs=Ok(8208)], "b "[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "c. ."[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "con.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "f.txt"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)], "nul.rs"[ft(sym,dir)=Ok((false, false)) attrs=Ok(8224)]
```

### Test `win::j_root_list_right_denied`

```text
test win::j_root_list_right_denied ... PROBE J1: icacls /deny RD: Ok((Some(0), "processed file: D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\j\\root  Successfully processed 1 files; Failed …
PROBE J2: read f.txt: Ok(1)
PROBE J2: list_dir root: Ok(1)
PROBE J3: DiskFs::new(root) now: Ok(())
PROBE J4: icacls /remove:d: Ok(Some(0))
PROBE J5: read f.txt after undo: Ok(1)
```

### Test `win::k_dotdot_after_junction_in_root_text`

```text
test win::k_dotdot_after_junction_in_root_text ... PROBE K1: mklink /J link -> a/real: status Some(0) stdout "Junction created for D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\k\\link <<===>> D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\k\\a\\real" stderr ""
PROBE K2: canonicalize(link/../b): Ok("\\\\?\\D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\k\\b")
PROBE K2: std::path::absolute(link/../b): Ok("D:\\a\\photocraft-study-review\\photocraft-study-review\\target\\tmp\\probe_t074\\k\\b")
PROBE K3: DiskFs::new(link/../b): Ok(())
PROBE K4: read f.txt: Ok("lexical")
```

### Summary

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
```

### Exit status

```text
PROBE T-074: exit status 0
```
