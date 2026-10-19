import styles from './editor.module.css';

interface Props {
  onEditAnyway: () => void;
}

/// Shown when the file holds formatting the editor would drop.
export function ReadOnlyBanner({ onEditAnyway }: Props) {
  return (
    <div className={styles.banner} role="status">
      <span>This script uses formatting Aside can&apos;t edit yet.</span>
      <button type="button" className={styles.bannerButton} onClick={onEditAnyway}>
        Edit Anyway
      </button>
    </div>
  );
}
