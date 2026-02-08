#!/bin/bash
# Claude Tabs Hook Script
# Called by Claude Code hooks (UserPromptSubmit, PermissionRequest, PostToolUse, Stop).
# Reads JSON from stdin, forwards session_id + hook_event_name to the app socket.

SOCKET="${CLAUDE_TABS_SOCKET:-$HOME/.claude-tabs/hook.sock}"
SESSION_ID="${CLAUDE_TABS_SESSION_ID}"

# If no session ID or socket, exit silently
if [ -z "$SESSION_ID" ] || [ ! -S "$SOCKET" ]; then
  cat >/dev/null  # Drain stdin
  echo '{}'
  exit 0
fi

# Parse JSON and send message in a single python call (avoids double cold-start)
python3 -c "
import sys, json, socket

data = json.load(sys.stdin)
event = data.get('hook_event_name', '')
claude_sid = data.get('session_id', '')

if event:
    msg = {'session_id': '$SESSION_ID', 'hook_event_name': event}
    if claude_sid:
        msg['claude_session_id'] = claude_sid
    # Forward notification type for Notification hooks
    if event == 'Notification':
        notif_type = data.get('notification_type') or data.get('type')
        if notif_type:
            msg['notification_type'] = notif_type
    # Forward is_interrupt for PostToolUseFailure hooks
    if event == 'PostToolUseFailure':
        msg['is_interrupt'] = data.get('is_interrupt', False)
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(1)
        sock.connect('$SOCKET')
        sock.sendall(json.dumps(msg).encode())
        sock.close()
    except:
        pass
" 2>/dev/null

echo '{}'
exit 0
