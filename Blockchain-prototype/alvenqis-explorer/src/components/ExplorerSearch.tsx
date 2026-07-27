import { FormEvent, useState } from "react";
import { useNavigate } from "react-router-dom";

export function ExplorerSearch() {
  const [query, setQuery] = useState("");
  const navigate = useNavigate();

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = query.trim();
    if (value) {
      navigate(`/search/${encodeURIComponent(value)}`);
    }
  }

  return (
    <form className="explorer-search" onSubmit={submit}>
      <label htmlFor="explorer-search">Search the candidate chain</label>
      <div className="search-row">
        <input
          id="explorer-search"
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Height, block hash, tx hash, or address"
          spellCheck={false}
          value={query}
        />
        <button type="submit">Search</button>
      </div>
    </form>
  );
}
