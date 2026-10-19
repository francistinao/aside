import { FileText } from 'lucide-react';
import type { Hit } from './vault';
import styles from './sidebar.module.css';

interface Props {
  query: string;
  hits: Hit[];
  openPath: string | null;
  onOpen: (path: string) => void;
}

export function SearchResults({ query, hits, openPath, onOpen }: Props) {
  if (hits.length === 0) {
    return <p className={styles.noResults}>No Results for &quot;{query}&quot;</p>;
  }
  return (
    <div className={styles.list} role="list" aria-label="Search results">
      {hits.map((hit) => (
        <div
          key={hit.path}
          role="listitem"
          className={`${styles.hit} ${hit.path === openPath ? styles.selected : ''}`}
          onClick={() => onOpen(hit.path)}
        >
          <FileText size={16} strokeWidth={1.5} className={styles.icon} aria-hidden="true" />
          <span className={styles.hitText}>
            <span className={styles.label}>{hit.name}</span>
            <span className={styles.snippet}>{hit.snippet}</span>
          </span>
        </div>
      ))}
    </div>
  );
}
