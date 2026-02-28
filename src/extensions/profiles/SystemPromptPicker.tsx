import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SystemPromptEntry } from "../../types/profile";

interface SystemPromptPickerProps {
  selected: string | null;
  onSelect: (name: string | null) => void;
}

export function SystemPromptPicker({ selected, onSelect }: SystemPromptPickerProps) {
  const [prompts, setPrompts] = useState<SystemPromptEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editContent, setEditContent] = useState("");
  const [saving, setSaving] = useState(false);

  const loadPrompts = useCallback(() => {
    invoke<SystemPromptEntry[]>("list_system_prompts")
      .then(setPrompts)
      .catch((err) => console.error("[SystemPromptPicker] Failed to load:", err))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    loadPrompts();
  }, [loadPrompts]);

  const handleNew = () => {
    setEditName("");
    setEditContent("");
    setEditing(true);
  };

  const handleEdit = async () => {
    if (!selected) return;
    try {
      const content = await invoke<string>("read_system_prompt", { name: selected });
      setEditName(selected);
      setEditContent(content);
      setEditing(true);
    } catch (err) {
      console.error("[SystemPromptPicker] Failed to read:", err);
    }
  };

  const handleSave = async () => {
    const name = editName.trim();
    if (!name || !editContent.trim()) return;
    setSaving(true);
    try {
      await invoke("save_system_prompt", { name, content: editContent });
      setEditing(false);
      loadPrompts();
      onSelect(name);
    } catch (err) {
      console.error("[SystemPromptPicker] Failed to save:", err);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!selected) return;
    try {
      await invoke("delete_system_prompt", { name: selected });
      onSelect(null);
      loadPrompts();
    } catch (err) {
      console.error("[SystemPromptPicker] Failed to delete:", err);
    }
  };

  if (loading) {
    return <span className="profiles-field-hint">Loading system prompts...</span>;
  }

  if (editing) {
    return (
      <div className="system-prompt-editor">
        <input
          className="profiles-field-input"
          type="text"
          placeholder="Prompt name (e.g. code-review)"
          value={editName}
          onChange={(e) => setEditName(e.target.value)}
        />
        <textarea
          className="profiles-field-textarea"
          placeholder="System prompt content (markdown)..."
          value={editContent}
          onChange={(e) => setEditContent(e.target.value)}
          rows={6}
        />
        <div className="system-prompt-editor-actions">
          <button
            className="profiles-cancel-btn"
            onClick={() => setEditing(false)}
          >
            Cancel
          </button>
          <button
            className="profiles-save-btn"
            onClick={handleSave}
            disabled={!editName.trim() || !editContent.trim() || saving}
          >
            {saving ? "Saving..." : "Save Prompt"}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="system-prompt-picker">
      <div className="system-prompt-picker-row">
        <select
          className="system-prompt-picker-select"
          value={selected || ""}
          onChange={(e) => onSelect(e.target.value || null)}
        >
          <option value="">None</option>
          {prompts.map((p) => (
            <option key={p.name} value={p.name}>
              {p.name}
            </option>
          ))}
        </select>
        <button
          className="system-prompt-action-btn"
          onClick={handleNew}
          title="New system prompt"
        >
          +
        </button>
        {selected && (
          <>
            <button
              className="system-prompt-action-btn"
              onClick={handleEdit}
              title="Edit selected prompt"
            >
              &#9998;
            </button>
            <button
              className="system-prompt-action-btn system-prompt-action-danger"
              onClick={handleDelete}
              title="Delete selected prompt"
            >
              &times;
            </button>
          </>
        )}
      </div>
      {selected && prompts.find((p) => p.name === selected) && (
        <div className="system-prompt-picker-preview">
          {prompts.find((p) => p.name === selected)!.preview}
        </div>
      )}
    </div>
  );
}
