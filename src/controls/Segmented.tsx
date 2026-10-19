import styles from './controls.module.css';

interface Option<T extends string> {
  value: T;
  label: string;
}

interface Props<T extends string> {
  value: T;
  options: Option<T>[];
  label: string;
  onChange: (value: T) => void;
}

/// DESIGN.md segmented control: 24 px, fill track, the selected segment raised.
export function Segmented<T extends string>({ value, options, label, onChange }: Props<T>) {
  return (
    <div className={styles.segmented} role="group" aria-label={label}>
      {options.map((option) => (
        <button key={option.value} type="button" aria-pressed={option.value === value} onClick={() => onChange(option.value)}>
          {option.label}
        </button>
      ))}
    </div>
  );
}
