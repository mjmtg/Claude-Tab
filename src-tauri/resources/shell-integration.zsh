# Claude Tabs Shell Integration
# Emits OSC 133 sequences for prompt boundaries

_claude_tabs_preexec() {
  printf '\e]133;C\e\\'  # Command execution starting
}

_claude_tabs_precmd() {
  printf '\e]133;D;%s\e\\' "$?"  # Command finished with exit code
  printf '\e]133;A\e\\'  # Prompt starting
}

# Hook into zsh
autoload -Uz add-zsh-hook
add-zsh-hook preexec _claude_tabs_preexec
add-zsh-hook precmd _claude_tabs_precmd

# Append OSC 133;B to prompt (marks end of prompt, start of input)
PROMPT="${PROMPT}%{\e]133;B\e\\%}"
