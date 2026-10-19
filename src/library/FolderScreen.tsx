import styles from './sidebar.module.css';
import screen from './screens.module.css';

interface Props {
  missing: boolean;
  onChoose: () => void;
}

/// First run, or the folder went away.
export function FolderScreen({ missing, onChoose }: Props) {
  return (
    <div className={screen.center}>
      <p className={screen.message}>
        {missing ? "The scripts folder can't be found. Choose it again to continue." : 'Choose a folder for your scripts.'}
      </p>
      <button type="button" className={`${styles.newScript} ${screen.primary}`} onClick={onChoose}>
        Choose Folder
      </button>
    </div>
  );
}
