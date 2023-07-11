pub const INTEGRATION_CODE: &str = r###"# To be eval'ed in the calling shell
  eval "$(
    if [ -n "$XDG_RUNTIME_DIR" ]; then
      runtime_dir="$XDG_RUNTIME_DIR/felix"
    elif [ -n "$TMPDIR" ]; then
      runtime_dir="$TMPDIR/felix"
    else
      runtime_dir=/tmp/felix
    fi

    # Clean up leftover LWD files
    # Option differences between BSD and GNU find implementations
    case "$(uname)" in
    Linux)
      find "$runtime_dir" -type f -and -mmin +0.02 -delete
      ;;
    Darwin)
      find "$runtime_dir" -type f -and -mtime +1s -delete
      ;;
    *) ;;
    esac

    cat << EOF
fx() {
  local RUNTIME_DIR="$runtime_dir"
  SHELL_PID=\$\$ command fx "\$@"

  if [ -f "\$RUNTIME_DIR/\$\$" ]; then
    cd "\$(cat "\$RUNTIME_DIR/\$\$")"
    rm "\$RUNTIME_DIR/\$\$"
  fi
}
EOF
  )"
"###;
