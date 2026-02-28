import React from "react";
import { ProfileInput } from "../../types/profile";

interface InputFieldProps {
  input: ProfileInput;
  value: string;
  onChange: (value: string) => void;
  onEnter?: () => void;
  autoFocusRef?: React.Ref<HTMLInputElement | HTMLTextAreaElement>;
}

export function InputField({ input, value, onChange, onEnter, autoFocusRef }: InputFieldProps) {
  if (input.input_type === "select" && input.options) {
    return (
      <select
        className="profiles-field-input"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      >
        <option value="">Select...</option>
        {input.options.map((opt) => (
          <option key={opt} value={opt}>{opt}</option>
        ))}
      </select>
    );
  }

  if (input.input_type === "list") {
    return (
      <textarea
        ref={autoFocusRef as React.Ref<HTMLTextAreaElement>}
        className="profiles-field-textarea"
        placeholder={input.placeholder || "One item per line..."}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        rows={4}
      />
    );
  }

  return (
    <input
      ref={autoFocusRef as React.Ref<HTMLInputElement>}
      className="profiles-field-input"
      type="text"
      placeholder={input.placeholder || ""}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => { if (e.key === "Enter" && onEnter) onEnter(); }}
    />
  );
}
