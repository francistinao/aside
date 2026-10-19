import { Search } from 'lucide-react';
import type { RefObject } from 'react';
import styles from './sidebar.module.css';

interface Props {
  value: string;
  onChange: (value: string) => void;
  inputRef: RefObject<HTMLInputElement | null>;
}

/// Escape clears the field; a second Escape on an empty field hands focus back to the tree.
export function SearchField({ value, onChange, inputRef }: Props) {
  return (
    <label className={styles.search}>
      <Search size={16} strokeWidth={1.5} className={styles.searchIcon} aria-hidden="true" />
      <input
        ref={inputRef}
        type="search"
        className={styles.searchInput}
        placeholder="Search"
        aria-label="Search scripts"
        value={value}
        spellCheck={false}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key !== 'Escape') return;
          event.preventDefault();
          if (value === '') event.currentTarget.blur();
          else onChange('');
        }}
      />
    </label>
  );
}
