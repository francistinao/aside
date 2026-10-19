import styles from './controls.module.css';

interface Props {
  checked: boolean;
  label: string;
  /// Shown on hover. The capture switch uses it for the status.
  title?: string;
  onChange: (checked: boolean) => void;
}

/// A 32 x 18 capsule with a white knob that slides over 160 ms.
export function Switch({ checked, label, title, onChange }: Props) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      title={title}
      className={styles.switch}
      onClick={() => onChange(!checked)}
    >
      <span className={styles.knob} />
    </button>
  );
}
