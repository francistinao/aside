import { Segmented } from '../controls/Segmented';
import { Switch } from '../controls/Switch';
import { isMac } from '../platform';
import type { Settings, SettingsPatch } from './api';
import { Row } from './rows';

interface Props {
  settings: Settings;
  onPatch: (patch: SettingsPatch) => void;
}

export function PrompterTab({ settings, onPatch }: Props) {
  return (
    <>
      <Row label="Follow">
        <Segmented
          value={settings.follow}
          label="Follow"
          options={[
            { value: 'snappy', label: 'Snappy' },
            { value: 'smooth', label: 'Smooth' },
            { value: 'floaty', label: 'Floaty' },
          ]}
          onChange={(follow) => onPatch({ follow })}
        />
      </Row>
      <Row label="Distance from pointer">
        <Segmented
          value={settings.distance}
          label="Distance from pointer"
          options={[
            { value: 'close', label: 'Close' },
            { value: 'normal', label: 'Normal' },
            { value: 'far', label: 'Far' },
          ]}
          onChange={(distance) => onPatch({ distance })}
        />
      </Row>
      <Row label="Flip at screen edges">
        <Switch checked={settings.flipAtEdges} label="Flip at screen edges" onChange={(flipAtEdges) => onPatch({ flipAtEdges })} />
      </Row>
      <Row label="Peek key" hint="Hold it to see what is under the bubble.">
        <Segmented
          value={settings.peekKey}
          label="Peek key"
          options={[
            { value: 'control', label: 'Control' },
            { value: 'option', label: isMac ? 'Option' : 'Alt' },
            { value: 'shift', label: 'Shift' },
          ]}
          onChange={(peekKey) => onPatch({ peekKey })}
        />
      </Row>
      <Row label="Fade while clicking">
        <Switch
          checked={settings.fadeWhileClicking}
          label="Fade while clicking"
          onChange={(fadeWhileClicking) => onPatch({ fadeWhileClicking })}
        />
      </Row>
      <Row label="Text size">
        <Segmented
          value={settings.textSize}
          label="Text size"
          options={[
            { value: 'small', label: 'Small' },
            { value: 'medium', label: 'Medium' },
            { value: 'large', label: 'Large' },
          ]}
          onChange={(textSize) => onPatch({ textSize })}
        />
      </Row>
    </>
  );
}
