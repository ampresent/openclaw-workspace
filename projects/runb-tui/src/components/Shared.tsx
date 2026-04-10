import React from 'react';
import { Box, Text, useInput, useStdout } from 'ink';

// ──────────────────────────────────────────────────────────────
// SuperDesign TUI Theme — terminal-adapted color system
// ──────────────────────────────────────────────────────────────
export const theme = {
  // primary palette (terminal ANSI)
  brand:    'cyan',        // brand accent (runb hexagon, active tab)
  accent:   'blueBright',  // secondary accent
  success:  'greenBright',
  warning:  'yellow',
  danger:   'redBright',
  muted:    'gray',        // dim text, separators
  subtle:   'white',       // body text

  // semantic
  active:   'greenBright', // running state
  idle:     'yellow',      // created state
  stopped:  'red',         // stopped state

  // unicode box drawing
  hline: '─',
  vline: '│',
  tl: '┌', tr: '┐', bl: '└', br: '┘',
  tee_l: '├', tee_r: '┤', tee_t: '┬', tee_b: '┴', cross: '┼',
};

// ──────────────────────────────────────────────────────────────
// TabBar — active tab gets a filled bottom border treatment
// ──────────────────────────────────────────────────────────────
interface TabBarProps {
  tabs: string[];
  active: number;
  onChange: (index: number) => void;
}

export const TabBar: React.FC<TabBarProps> = ({ tabs, active, onChange }) => {
  useInput((input, key) => {
    if (key.tab) onChange((active + 1) % tabs.length);
    const num = parseInt(input);
    if (num >= 1 && num <= tabs.length) onChange(num - 1);
  });

  return (
    <Box>
      {tabs.map((tab, i) => {
        const isActive = i === active;
        return (
          <Box key={i} marginRight={1}>
            <Text
              bold={isActive}
              backgroundColor={isActive ? theme.brand : undefined}
              color={isActive ? 'black' : theme.muted}
            >
              {isActive ? ` ${tab} ` : ` ${tab} `}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
};

// ──────────────────────────────────────────────────────────────
// ShortcutBar — lazygit-style bottom status bar
// ──────────────────────────────────────────────────────────────
interface ShortcutBarProps {
  shortcuts: Array<{ key: string; label: string }>;
  extra?: string;
}

export const ShortcutBar: React.FC<ShortcutBarProps> = ({ shortcuts, extra }) => (
  <Box marginTop={1}>
    {shortcuts.map((s, i) => (
      <Box key={i} marginRight={3}>
        <Text backgroundColor={theme.muted} color="black" bold>
          {` ${s.key} `}
        </Text>
        <Text color={theme.subtle}> {s.label}</Text>
      </Box>
    ))}
    {extra && (
      <Box flexGrow={1} justifyContent="flex-end">
        <Text color={theme.muted} dimColor>{extra}</Text>
      </Box>
    )}
  </Box>
);

// ──────────────────────────────────────────────────────────────
// BoxPanel — wraps content in a unicode box
// ──────────────────────────────────────────────────────────────
interface BoxPanelProps {
  title?: string;
  children: React.ReactNode;
  width?: number;
}

export const BoxPanel: React.FC<BoxPanelProps> = ({ title, children, width }) => {
  const { stdout } = useStdout();
  const w = width || (stdout.columns || 80) - 4;
  const innerW = w - 2; // minus the two vlines

  return (
    <Box flexDirection="column">
      <Text color={theme.muted}>
        {theme.tl}{title ? `${theme.hline} ` : ''}{title || ''}{title ? ' ' : ''}{theme.hline.repeat(Math.max(0, innerW - (title ? title.length + 3 : 0)))}{theme.tr}
      </Text>
      {children}
      <Text color={theme.muted}>
        {theme.bl}{theme.hline.repeat(innerW)}{theme.br}
      </Text>
    </Box>
  );
};

// ──────────────────────────────────────────────────────────────
// SplitPane — side-by-side layout
// ──────────────────────────────────────────────────────────────
interface SplitPaneProps {
  left: React.ReactNode;
  right: React.ReactNode;
  leftPercent?: number;
}

export const SplitPane: React.FC<SplitPaneProps> = ({ left, right, leftPercent = 0.55 }) => (
  <Box flexGrow={1} flexShrink={0} flexDirection="row" overflow="hidden">
    <Box
      flexDirection="column"
      flexBasis={`${Math.round(leftPercent * 100)}%`}
      flexShrink={0}
      overflow="hidden"
    >
      {left}
    </Box>
    <Box
      flexDirection="column"
      flexGrow={1}
      overflow="hidden"
      paddingLeft={2}
    >
      {right}
    </Box>
  </Box>
);

// ──────────────────────────────────────────────────────────────
// ListItem — with aligned indicator
// ──────────────────────────────────────────────────────────────
interface ListItemProps {
  label: string;
  selected: boolean;
  indicator?: string;
  color?: string;
}

export const ListItem: React.FC<ListItemProps> = ({ label, selected, indicator, color }) => (
  <Box>
    <Text color={selected ? theme.brand : theme.muted}>
      {selected ? '▸ ' : '  '}
    </Text>
    {indicator && <Text color={color || theme.subtle}>{indicator} </Text>}
    <Text color={selected ? 'white' : color || theme.subtle} bold={selected}>
      {label}
    </Text>
  </Box>
);

// ──────────────────────────────────────────────────────────────
// KeyValueList — structured detail display
// ──────────────────────────────────────────────────────────────
interface KeyValueProps {
  items: Array<{ key: string; value: string; color?: string }>;
  labelWidth?: number;
}

export const KeyValueList: React.FC<KeyValueProps> = ({ items, labelWidth }) => {
  const maxLen = labelWidth || Math.max(...items.map(i => i.key.length), 6);
  return (
    <Box flexDirection="column">
      {items.map((item, i) => (
        <Box key={i}>
          <Text color={theme.muted}>{item.key.padStart(maxLen)}  </Text>
          <Text color={item.color || theme.subtle}>{item.value}</Text>
        </Box>
      ))}
    </Box>
  );
};

// ──────────────────────────────────────────────────────────────
// SectionHeader — grouped section title
// ──────────────────────────────────────────────────────────────
interface SectionHeaderProps {
  title: string;
  count?: number;
}

export const Header: React.FC<SectionHeaderProps> = ({ title, count }) => (
  <Box marginBottom={1}>
    <Text bold color="white">
      {title}
      {count !== undefined && <Text color={theme.muted}> ({count})</Text>}
    </Text>
  </Box>
);

export const SectionHeader: React.FC<SectionHeaderProps> = ({ title, count }) => (
  <Box marginTop={1} marginBottom={1}>
    <Text color={theme.brand} bold>── </Text>
    <Text bold color="white">{title}</Text>
    {count !== undefined && <Text color={theme.muted}> ({count})</Text>}
    <Text color={theme.brand} bold> ──</Text>
  </Box>
);

// ──────────────────────────────────────────────────────────────
// EmptyState
// ──────────────────────────────────────────────────────────────
interface EmptyStateProps {
  message: string;
  hint?: string;
}

export const EmptyState: React.FC<EmptyStateProps> = ({ message, hint }) => (
  <Box flexDirection="column" alignItems="center" justifyContent="center" flexGrow={1}>
    <Text color={theme.muted}>{message}</Text>
    {hint && <Text color={theme.muted} dimColor>{hint}</Text>}
  </Box>
);

// ──────────────────────────────────────────────────────────────
// Separator — horizontal line
// ──────────────────────────────────────────────────────────────
export const Separator: React.FC<{ char?: string }> = ({ char }) => {
  const { stdout } = useStdout();
  const w = (stdout.columns || 80) - 4;
  return <Text color={theme.muted}>{(char || theme.hline).repeat(w)}</Text>;
};

// ──────────────────────────────────────────────────────────────
// Hooks
// ──────────────────────────────────────────────────────────────
export function useTerminalWidth(): number {
  const { stdout } = useStdout();
  return stdout.columns || 80;
}
