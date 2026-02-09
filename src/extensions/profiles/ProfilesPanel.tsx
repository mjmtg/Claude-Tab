import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Profile, ProfileLaunchRequest } from "../../types/profile";
import { ProfileEditor } from "./ProfileEditor";

let showProfiles = false;
let profilesListeners: Array<() => void> = [];

function notifyProfiles() {
  profilesListeners.forEach((l) => l());
}

export function setProfilesPanelVisible(visible: boolean) {
  showProfiles = visible;
  notifyProfiles();
}

export function getProfilesPanelVisible() {
  return showProfiles;
}

export function ProfilesPanel() {
  const [visible, setVisible] = useState(false);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [inputValues, setInputValues] = useState<Record<string, string>>({});
  const [workingDir, setWorkingDir] = useState("");
  const [showEditor, setShowEditor] = useState(false);
  const [editingProfile, setEditingProfile] = useState<Profile | null>(null);
  const [launching, setLaunching] = useState(false);

  useEffect(() => {
    const update = () => {
      const wasVisible = visible;
      setVisible(showProfiles);
      if (showProfiles) {
        loadProfiles();
        setExpandedId(null);
        setShowEditor(false);
        setEditingProfile(null);
      } else if (wasVisible) {
        // Return focus to terminal when profiles closes
        const terminal = document.querySelector(".xterm-helper-textarea") as HTMLTextAreaElement;
        if (terminal) terminal.focus();
      }
    };
    profilesListeners.push(update);
    return () => {
      profilesListeners = profilesListeners.filter((l) => l !== update);
    };
  }, [visible]);

  const loadProfiles = useCallback(async () => {
    try {
      const list = await invoke<Profile[]>("list_profiles");
      setProfiles(list);
    } catch (err) {
      console.error("[Profiles] Failed to load:", err);
    }
  }, []);

  const close = () => {
    showProfiles = false;
    notifyProfiles();
  };

  const handleCardClick = (profile: Profile) => {
    // If no inputs needed and fixed directory, launch immediately
    const needsInputs = profile.inputs.length > 0;
    const needsDir =
      !profile.working_directory ||
      profile.working_directory.type === "prompt";

    if (!needsInputs && !needsDir) {
      launchProfile(profile, {}, profile.working_directory?.path);
      return;
    }

    if (expandedId === profile.id) {
      setExpandedId(null);
    } else {
      setExpandedId(profile.id);
      // Initialize input values with defaults
      const defaults: Record<string, string> = {};
      for (const input of profile.inputs) {
        if (input.default) {
          defaults[input.key] = input.default;
        }
      }
      setInputValues(defaults);
      setWorkingDir(
        profile.working_directory?.type === "fixed"
          ? profile.working_directory.path || ""
          : ""
      );
    }
  };

  const launchProfile = async (
    profile: Profile,
    values: Record<string, string>,
    dir?: string
  ) => {
    setLaunching(true);
    try {
      const request: ProfileLaunchRequest = {
        profile_id: profile.id,
        input_values: values,
        working_directory: dir,
      };
      await invoke("launch_profile", { request });
      close();
    } catch (err) {
      console.error("[Profiles] Launch failed:", err);
    } finally {
      setLaunching(false);
    }
  };

  const handleLaunch = (profile: Profile) => {
    // Validate required inputs
    for (const input of profile.inputs) {
      if (input.required && !inputValues[input.key]?.trim()) {
        return;
      }
    }
    const needsDir =
      !profile.working_directory ||
      profile.working_directory.type === "prompt";
    if (needsDir && !workingDir.trim()) {
      return;
    }
    launchProfile(profile, inputValues, workingDir || undefined);
  };

  const handleBrowseDir = async () => {
    const dir = await open({
      title: "Select Working Directory",
      directory: true,
    });
    if (dir) {
      setWorkingDir(dir);
    }
  };

  const handleInputChange = (key: string, value: string) => {
    setInputValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleNewProfile = () => {
    setEditingProfile(null);
    setShowEditor(true);
  };

  const handleEditProfile = (e: React.MouseEvent, profile: Profile) => {
    e.stopPropagation();
    setEditingProfile(profile);
    setShowEditor(true);
  };

  const handleDeleteProfile = async (e: React.MouseEvent, profileId: string) => {
    e.stopPropagation();
    try {
      await invoke("delete_profile", { profileId });
      loadProfiles();
    } catch (err) {
      console.error("[Profiles] Delete failed:", err);
    }
  };

  const handleEditorSave = async (profile: Profile) => {
    try {
      await invoke("save_profile", { profile });
      setShowEditor(false);
      loadProfiles();
    } catch (err) {
      console.error("[Profiles] Save failed:", err);
    }
  };

  const handleEditorClose = () => {
    setShowEditor(false);
  };

  if (!visible) return null;

  if (showEditor) {
    return (
      <div className="profiles-backdrop" onClick={close} role="presentation">
        <div className="profiles-panel profiles-panel-wide" onClick={(e) => e.stopPropagation()} role="dialog" aria-label="Edit profile" aria-modal="true">
          <ProfileEditor
            profile={editingProfile}
            onSave={handleEditorSave}
            onClose={handleEditorClose}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="profiles-backdrop" onClick={close} role="presentation">
      <div className="profiles-panel" onClick={(e) => e.stopPropagation()} role="dialog" aria-label="Profiles" aria-modal="true">
        <div className="profiles-header">
          <span className="profiles-title">Profiles</span>
          <button className="profiles-close" onClick={close} aria-label="Close profiles">
            &times;
          </button>
        </div>
        <div className="profiles-list">
          {profiles.map((profile) => (
            <div key={profile.id} className="profiles-card-wrapper">
              <div
                className={`profiles-card ${expandedId === profile.id ? "profiles-card-expanded" : ""}`}
                onClick={() => handleCardClick(profile)}
              >
                <div className="profiles-card-header">
                  <div className="profiles-card-info">
                    <span className="profiles-card-name">{profile.name}</span>
                    {profile.description && (
                      <span className="profiles-card-desc">{profile.description}</span>
                    )}
                  </div>
                  <div className="profiles-card-actions">
                    <button
                      className="profiles-card-action"
                      onClick={(e) => handleEditProfile(e, profile)}
                      title="Edit"
                    >
                      &#9998;
                    </button>
                    <button
                      className="profiles-card-action profiles-card-action-danger"
                      onClick={(e) => handleDeleteProfile(e, profile.id)}
                      title="Delete"
                    >
                      &times;
                    </button>
                  </div>
                </div>
              </div>
              {expandedId === profile.id && (
                <div className="profiles-launch-form">
                  {profile.inputs.map((input) => (
                    <div key={input.key} className="profiles-field">
                      <label className="profiles-field-label">
                        {input.label}
                        {input.required && <span className="profiles-required">*</span>}
                      </label>
                      {input.input_type === "select" && input.options ? (
                        <select
                          className="profiles-field-input"
                          value={inputValues[input.key] || ""}
                          onChange={(e) => handleInputChange(input.key, e.target.value)}
                        >
                          <option value="">Select...</option>
                          {input.options.map((opt) => (
                            <option key={opt} value={opt}>{opt}</option>
                          ))}
                        </select>
                      ) : (
                        <input
                          className="profiles-field-input"
                          type="text"
                          placeholder={input.placeholder || ""}
                          value={inputValues[input.key] || ""}
                          onChange={(e) => handleInputChange(input.key, e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleLaunch(profile);
                          }}
                        />
                      )}
                    </div>
                  ))}
                  {(!profile.working_directory ||
                    profile.working_directory.type === "prompt") && (
                    <div className="profiles-field">
                      <label className="profiles-field-label">Working Directory</label>
                      <div className="profiles-dir-row">
                        <input
                          className="profiles-field-input profiles-dir-input"
                          type="text"
                          placeholder="~/my-project"
                          value={workingDir}
                          onChange={(e) => setWorkingDir(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleLaunch(profile);
                          }}
                        />
                        <button
                          className="profiles-dir-browse"
                          onClick={handleBrowseDir}
                          title="Browse"
                        >
                          ...
                        </button>
                      </div>
                    </div>
                  )}
                  <button
                    className="profiles-launch-btn"
                    onClick={() => handleLaunch(profile)}
                    disabled={launching}
                  >
                    {launching ? "Launching..." : "Launch"}
                  </button>
                </div>
              )}
            </div>
          ))}
          {profiles.length === 0 && (
            <div className="profiles-empty">
              No profiles yet. Create one to get started.
            </div>
          )}
        </div>
        <div className="profiles-footer">
          <button className="profiles-new-btn" onClick={handleNewProfile}>
            + New Profile
          </button>
        </div>
      </div>
    </div>
  );
}
