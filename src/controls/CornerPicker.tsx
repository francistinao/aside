import styles from './controls.module.css';

export type Corner = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

const CORNERS: { value: Corner; label: string }[] = [
  { value: 'top-left', label: 'Top Left' },
  { value: 'top-right', label: 'Top Right' },
  { value: 'bottom-left', label: 'Bottom Left' },
  { value: 'bottom-right', label: 'Bottom Right' },
];

interface Props {
  value: Corner;
  onChange: (value: Corner) => void;
}

/// A 2 x 2 grid of small squares, the chosen one in accent.
export function CornerPicker({ value, onChange }: Props) {
  return (
    <div className={styles.corners} role="radiogroup" aria-label="Position">
      {CORNERS.map((corner) => (
        <button
          key={corner.value}
          type="button"
          role="radio"
          aria-checked={corner.value === value}
          aria-label={corner.label}
          title={corner.label}
          onClick={() => onChange(corner.value)}
        />
      ))}
    </div>
  );
}
