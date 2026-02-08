# Claude Tabs Shell Integration for Bash
# Emits OSC 133 sequences for prompt boundaries

_claude_tabs_preexec() {
  printf '\e]133;C\e\\'  # Command execution starting
}

_claude_tabs_precmd() {
  printf '\e]133;D;%s\e\\' "$?"  # Command finished with exit code
  printf '\e]133;A\e\\'  # Prompt starting
}

# Hook into bash using PROMPT_COMMAND and DEBUG trap
trap '_claude_tabs_preexec' DEBUG

_original_prompt_command="${PROMPT_COMMAND}"
PROMPT_COMMAND='_claude_tabs_precmd; eval "$_original_prompt_command"'

# Append OSC 133;B to prompt (marks end of prompt, start of input)
PS1="${PS1}\[\e]133;B\e\\\]"
