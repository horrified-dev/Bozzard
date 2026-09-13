"""User-game packaging orchestration. Cooking lives in bozzard-project, shared with the editor."""
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import zipfile


def export_game(args, root):
    if args.verify_first_trail and not args.verify:
        raise ValueError("--verify-first-trail requires --verify")
    suffix = ".exe" if platform.system() == "Windows" else ""
    player = root / "target" / args.profile / f"bozzard-player{suffix}"
    destination = (args.export_dir or root / "dist" / f"first-game-{platform.system().lower()}-{platform.machine().lower()}").resolve()
    archive_path = destination.parent / (destination.name + ".zip")
    if archive_path.exists():
        raise FileExistsError(f"Archive already exists: {archive_path}")
    subprocess.run([str(player), "--export-project", str(args.project.resolve()),
                    "--export-dir", str(destination)], check=True)
    # Stable ordering, timestamps and permissions; identical inputs yield identical ZIPs.
    with zipfile.ZipFile(archive_path, "x", zipfile.ZIP_DEFLATED) as archive:
        for file in sorted(destination.rglob("*")):
            if file.is_file():
                entry = zipfile.ZipInfo((Path(destination.name) / file.relative_to(destination)).as_posix(), (1980, 1, 1, 0, 0, 0))
                entry.create_system = 3
                entry.external_attr = (file.stat().st_mode & 0xFFFF) << 16
                entry.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(entry, file.read_bytes())
    print(f"game_folder={destination}\ngame_archive={archive_path}", flush=True)
    if args.verify:
        verify_export(archive_path, destination.name, args, root)


def verify_export(archive_path, folder, args, root):
    graphics = ["--backend", args.backend] if args.backend else []
    if args.hardware:
        graphics.append("--hardware")
    if args.software:
        graphics.append("--software")
    with tempfile.TemporaryDirectory(prefix="bozzard exported game ") as temporary:
        temporary = Path(temporary)
        with zipfile.ZipFile(archive_path) as archive:
            archive.extractall(temporary)
            for entry in archive.infolist():
                if entry.external_attr >> 16:
                    (temporary / entry.filename).chmod(entry.external_attr >> 16)
        game = temporary / "Renamed game with spaces"
        (temporary / folder).rename(game)
        manifest = json.loads((game / "package.json").read_text())
        for relative, size in manifest["files"].items():
            file = game / relative
            if not file.is_file() or file.stat().st_size != size:
                raise RuntimeError(f"Package inventory mismatch: {relative}")
        executable = game / manifest["executable"]
        cwd = temporary / "empty working directory"
        cwd.mkdir()
        no_tools = temporary / "no build tools"
        no_tools.mkdir()
        env = dict(os.environ, PATH=str(no_tools))
        # Launch by absolute executable path, with no scene/project argument, from an
        # unrelated directory and a PATH containing neither Rust nor Python.
        def run(*arguments):
            subprocess.run([str(executable), *arguments], cwd=cwd, env=env, check=True, timeout=180)
        run("--write-scene", str(cwd / "snapshot.json"))
        if args.verify_first_trail:
            run("--verify-first-trail")
        run("--smoke", "--output", str(root / "work/export-package-smoke"), *graphics)
        if args.window:
            run(*(["--verify-first-trail", "--frames", "340"] if args.verify_first_trail else ["--frames", "3"]), *graphics)
        if any(cwd.glob("work/*")):
            raise RuntimeError("The game unexpectedly wrote development files")
    print("game_export_ok: extracted, renamed, no scene arguments, empty cwd/PATH, assets and native runtime verified", flush=True)
