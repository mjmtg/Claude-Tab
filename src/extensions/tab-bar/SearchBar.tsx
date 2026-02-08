import React, { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ClaudeSession } from "./types";

interface SearchResult {
  session_id: string;
  title: string;
  project: string;
  snippet: string;
}

interface SearchBarProps {
  onResultClick: (result: { session_id: string }) => void;
  inputRef?: React.RefObject<HTMLInputElement | null>;
}

export function SearchBar({ onResultClick, inputRef }: SearchBarProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [isOpen, setIsOpen] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const doSearch = useCallback(async (q: string) => {
    if (q.trim().length < 2) {
      setResults([]);
      setIsOpen(false);
      return;
    }
    try {
      const sessions = await invoke<ClaudeSession[]>("list_claude_sessions", {
        filter: {
          search_query: q.trim(),
          limit: 10,
          include_hidden: false,
        },
      });
      const mapped: SearchResult[] = sessions.map((s) => ({
        session_id: s.session_id,
        title: s.summary || s.first_prompt || `Session ${s.session_id.slice(0, 8)}`,
        project: s.project_path.split("/").filter(Boolean).pop() || s.project_path,
        snippet: s.first_prompt || "",
      }));
      setResults(mapped);
      setIsOpen(mapped.length > 0);
    } catch (err) {
      console.error("[SearchBar] search failed:", err);
      setResults([]);
      setIsOpen(false);
    }
  }, []);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => doSearch(val), 250);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      setQuery("");
      setResults([]);
      setIsOpen(false);
    }
  };

  const handleResultClick = (result: SearchResult) => {
    setIsOpen(false);
    setQuery("");
    onResultClick({ session_id: result.session_id });
  };

  // Close dropdown on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div className="search-bar-container" ref={containerRef}>
      <input
        ref={inputRef}
        className="search-bar-input"
        type="text"
        placeholder="Search sessions..."
        value={query}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onFocus={() => { if (results.length > 0) setIsOpen(true); }}
      />
      {isOpen && (
        <div className="search-bar-dropdown">
          {results.map((r, idx) => (
            <div
              key={`${r.session_id}-${idx}`}
              className="search-bar-result"
              onClick={() => handleResultClick(r)}
            >
              <div className="search-bar-result-title">
                {truncate(r.title, 60)}
              </div>
              <div className="search-bar-result-project">{r.project}</div>
              {r.snippet && r.snippet !== r.title && (
                <div className="search-bar-result-snippet">
                  <HighlightedSnippet text={truncate(r.snippet, 100)} query={query} />
                </div>
              )}
              <div className="search-bar-result-action">Resume session</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function truncate(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return str.slice(0, maxLen - 3) + "...";
}

function HighlightedSnippet({ text, query }: { text: string; query: string }) {
  if (!query.trim()) return <>{text}</>;

  const escapedQuery = query.trim().replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escapedQuery})`, "gi");
  const parts = text.split(regex);

  return (
    <>
      {parts.map((part, i) =>
        i % 2 === 1 ? <mark key={i}>{part}</mark> : <span key={i}>{part}</span>
      )}
    </>
  );
}
