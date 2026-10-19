#!/usr/bin/env python3
"""Build, check, test, or run game-server natively on explicit desktop/local hosts."""

import argparse
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
COMMON_SCRIPTS = SCRIPTS.parent.parent / "game-engine/scripts"
BINARIES = {
    "game-server": "server",
    "game-server-admin": "server",
    "game-server-site-export": "server",
    "bot-map": "server",
    "game-cli": "client",
    "los-bake": "los",
}
DEFAULT_BINARIES = ("game-server", "game-server-admin", "game-server-site-export", "game-cli")
GROUND_GAPS = Path("crates/server/src/ground_gaps.tsv")
ASSET_CWD = Path("/home/osso-test/.local/share/game-server/assets")
ENVIRONMENT_NAMES = (
    "GAME_SERVER_GROUND_DIR",
    "GAME_SERVER_DB_PATH",
    "GAME_SERVER_ADMIN_SOCKET",
    "GAME_SERVER_BIND_IP",
    "GAME_SERVER_PORT",
    "GAME_SERVER_DAILY_RESET_HOUR_UTC",
)


def load_common():
    scripts = Path(os.environ.get("BUILD_HOST_SCRIPTS", COMMON_SCRIPTS)).resolve()
    sys.path.insert(0, str(scripts))
    spec = importlib.util.spec_from_file_location(
        "depot_build", scripts / "depot-build.py"
    )
    depot = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(depot)
    import native_build_hosts

    return depot, native_build_hosts


def copy_source(repo, relative, destination):
    source = repo / relative
    if source.is_symlink() or any(
        (repo / parent).is_symlink() for parent in relative.parents
    ):
        raise ValueError(f"unsupported source symlink: {source}")
    if not source.is_file():
        raise FileNotFoundError(f"missing build dependency: {source}")
    target = destination / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)


def snapshot(root, context, depot):
    protocol = depot.sibling_repo(root, "shared-protocol")
    for name, repo in (("game-server", root), ("shared-protocol", protocol)):
        if not repo.is_dir():
            raise FileNotFoundError(f"missing build dependency: {repo}")
        depot.snapshot_repo(repo, context / name, (".",), name)
        for config in (Path(".cargo/config"), Path(".cargo/config.toml")):
            if (repo / config).exists() or (repo / config).is_symlink():
                copy_source(repo, config, context / name)
    if GROUND_GAPS in depot.git_files(root, (".",), True):
        copy_source(root, GROUND_GAPS, context / "game-server")
    for relative in (
        "game-server/Cargo.toml",
        "game-server/Cargo.lock",
        "shared-protocol/Cargo.toml",
    ):
        if not (context / relative).is_file():
            raise FileNotFoundError(f"missing build dependency: {relative}")


def has_package_selection(arguments):
    # Only Cargo options count; the test executable's tail is opaque.
    options = arguments[: arguments.index("--")] if "--" in arguments else arguments
    return any(
        arg in {"-p", "--package", "--workspace"}
        or arg.startswith(("--package=", "-p"))
        for arg in options
    )


def cargo_arguments(args, tail):
    if args.test or args.check:
        selection = [] if has_package_selection(tail) else ["--workspace"]
        features = ["--features", args.features] if args.features else []
        return ["test" if args.test else "check", *selection, *features, *tail]
    binaries = [args.binary] if args.binary else list(DEFAULT_BINARIES)
    packages = list(dict.fromkeys(BINARIES[binary] for binary in binaries))
    arguments = ["build"]
    for package in packages:
        arguments.extend(["-p", package])
    for binary in binaries:
        arguments.extend(["--bin", binary])
    if args.features:
        arguments.extend(["--features", args.features])
    return arguments


def runtime_environment(host, key, running):
    environment = {
        name: os.environ[name] for name in ENVIRONMENT_NAMES if name in os.environ
    }
    if host == "desktop" and running:
        state = Path("/home/osso-test/.local/share/game-server") / key
        defaults = {
            "GAME_SERVER_GROUND_DIR": "/home/osso-test/game-engine-trial/ground",
            "GAME_SERVER_DB_PATH": str(state / "game.redb"),
            "GAME_SERVER_ADMIN_SOCKET": str(state / "admin.sock"),
        }
        environment = defaults | environment
    return environment


def build(args, tail, depot, native):
    root = args.root.resolve(strict=True)
    host = depot.select_build_host(args.build_host)
    if args.release and host == "native":
        raise ValueError(
            "release artifacts use the bookworm container: --build-host local|desktop"
        )
    if args.release:
        if args.test or args.check or args.run:
            raise ValueError(
                "container release exports support --bin and --features, not test/check/run"
            )
        spec = importlib.util.spec_from_file_location(
            "desktop_server_build",
            Path(depot.__file__).with_name("desktop-server-build.py"),
        )
        exporter = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(exporter)
        exporter.build(
            root, host=host, release=True, binary=args.binary, features=args.features
        )
        return 0
    if host == "native":
        from native_cargo import native_slot, run_cargo

        lock, _, _ = depot.locked_checkout(root)
        with lock, native_slot():
            source = depot.native_source_root(root, "game-server", ("shared-protocol",))
            code = run_cargo(root, cargo_arguments(args, tail), source_root=source)
        if code or not args.run:
            return code
        # Runtime owns its process lifetime, but does not occupy a compilation slot.
        return native.execute(
            root,
            "server-native",
            "game-server",
            [],
            "local",
            runtime_args=tail,
            binary=args.binary,
            root=root,
            runtime_cwd=root,
            environment=runtime_environment(host, "server-native", True),
            build=False,
        )
    cache, checkout_key = depot.prepare_checkout_cache(root)
    key = "server-" + checkout_key
    runtime_args = tail if args.run else None
    runtime_cwd = (root if host == "local" else ASSET_CWD) if args.run else None
    with tempfile.TemporaryDirectory(prefix="server-native-", dir=cache) as work:
        context = Path(work)
        # Local compilation must read the original paths, including its local data.
        if host == "desktop":
            snapshot(root, context, depot)
        print(f"native game-server host={host} key={key}", flush=True)
        return native.execute(
            context,
            key,
            "game-server",
            cargo_arguments(args, tail),
            host,
            runtime_args=runtime_args,
            binary=args.binary,
            release=args.release,
            environment=runtime_environment(host, key, args.run),
            root=root,
            runtime_cwd=runtime_cwd,
        )


def parse_arguments(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=SCRIPTS.parent)
    parser.add_argument("--build-host", choices=("desktop", "local", "native"))
    parser.add_argument("--save-build-host", choices=("desktop", "local", "native"))
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--bin", dest="binary", choices=BINARIES)
    parser.add_argument("--features", help="Cargo feature CSV")
    parser.add_argument("--run", action="store_true")
    parser.add_argument(
        "--check", action="store_true", help="check workspace; accepts Cargo arguments"
    )
    parser.add_argument(
        "--test",
        action="store_true",
        help="last adapter option; forwards all remaining Cargo arguments",
    )
    separator = argv.index("--") if "--" in argv else len(argv)
    if "--test" in argv[:separator]:
        index = argv.index("--test")
        args = parser.parse_args(argv[: index + 1])
        tail = argv[index + 1 :]
    else:
        args, extra = parser.parse_known_args(argv[:separator])
        if args.check:
            tail = extra + argv[separator:]
        else:
            if extra:
                parser.error("unrecognized arguments: " + " ".join(extra))
            tail = argv[separator + 1 :] if separator < len(argv) else []
    if args.save_build_host and (
        args.build_host
        or args.release
        or args.binary
        or args.features
        or args.run
        or args.test
        or args.check
        or tail
    ):
        parser.error("--save-build-host must be used alone (apart from --root)")
    if (args.test and args.check) or (
        (args.test or args.check) and (args.run or args.binary)
    ):
        parser.error("test/check cannot combine with run/bin or each other")
    if tail and not (args.test or args.check or args.run):
        parser.error("runtime arguments require --run")
    if args.run and args.binary is None:
        args.binary = "game-server"
    return args, tail


def main(argv=None):
    args, tail = parse_arguments(sys.argv[1:] if argv is None else argv)
    try:
        depot, native = load_common()
        if args.save_build_host:
            depot.save_build_host(args.save_build_host)
            return 0
        return build(args, tail, depot, native)
    except (OSError, ValueError, subprocess.CalledProcessError, ImportError) as error:
        print(f"Native game-server failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
