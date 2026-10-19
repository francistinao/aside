import styles from './sidebar.module.css';
import screen from './screens.module.css';

interface Props {
  onNewScript: () => void;
}

/// Copy deck in docs/PRODUCT.md: "No Scripts" with one primary action.
export function EmptyLibrary({ onNewScript }: Props) {
  return (
    <div className={screen.center}>
      <h1 className={screen.title}>No Scripts</h1>
      <button type="button" className={`${styles.newScript} ${screen.primary}`} onClick={onNewScript}>
        New Script
      </button>
    </div>
  );
}
