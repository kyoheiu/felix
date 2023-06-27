pub const INTEGRATION_CODE: &str = r###"# To be eval'ed in the calling shell
  eval "$(
    if [ -n "$XDG_RUNTIME_DIR" ]; then
      runtime_dir="$XDG_RUNTIME_DIR/felix"
    elif [ -n "$TMPDIR" ]; then
      runtime_dir="$TMPDIR/felix"
    else
      runtime_dir=/tmp/felix
    fi

    # Option differences between BSD and GNU find implementations
    case "$(uname)" in
    Linux)
      file_age_option='-mmin +$(echo 1/60 | bc -l)'
      ;;
    Darwin)
      file_age_option='-mtime +1s'
      ;;
    *) ;;
    esac

    cat << EOF
fx() {
  local RUNTIME_DIR="$runtime_dir"
  SHELL_PID=\$\$ command fx "\$@"

  if [ -f "\$RUNTIME_DIR/\$\$" ]; then
    cd "\$(cat "\$RUNTIME_DIR/\$\$")"
  fi

  # Finally, clean up current and leftover lwd files
  find "\$RUNTIME_DIR" -type f -and \( $file_age_option -or -name \$\$ \) -delete
}
EOF
  )"
"###;
