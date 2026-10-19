import styles from './controls.module.css';

interface Props {
  value: number;
  min: number;
  max: number;
  step: number;
  label: string;
  onChange: (value: number) => void;
}

/// A 4 px track, accent fill up to the knob, 18 px knob.
export function Slider({ value, min, max, step, label, onChange }: Props) {
  const fraction = max > min ? (value - min) / (max - min) : 0;
  return (
    <input
      type="range"
      className={styles.slider}
      style={{ '--fill-fraction': fraction } as React.CSSProperties}
      aria-label={label}
      min={min}
      max={max}
      step={step}
      value={value}
      onChange={(event) => onChange(Number(event.target.value))}
    />
  );
}
