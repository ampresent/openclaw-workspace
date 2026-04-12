import React from 'react';
import { Box, Text } from 'ink';
import Gradient from 'ink-gradient';

export default function Header({ activeTab, tabs }) {
  return (
    <Box flexDirection="column" borderStyle="double" borderColor="cyan" paddingX={1}>
      <Box>
        <Gradient name="rainbow">
          <Text bold>  ███╗   ██╗██╗██╗  ██╗    ██████╗  █████╗ ████████╗ ██████╗██╗  ██╗    ██╗    ██╗ █████╗ ████████╗ ██████╗██╗  ██╗</Text>
        </Gradient>
      </Box>
      <Box>
        <Gradient name="rainbow">
          <Text bold>  ████╗  ██║██║╚██╗██╔╝    ██╔══██╗██╔══██╗╚══██╔══╝██╔════╝██║  ██║    ██║    ██║██╔══██╗╚══██╔══╝██╔════╝██║  ██║</Text>
        </Gradient>
      </Box>
      <Box>
        <Gradient name="rainbow">
          <Text bold>  ██╔██╗ ██║██║ ╚███╔╝     ██████╔╝███████║   ██║   ██║     ███████║    ██║ █╗ ██║███████║   ██║   ██║     ███████║</Text>
        </Gradient>
      </Box>
      <Box>
        <Gradient name="rainbow">
          <Text bold>  ██║╚██╗██║██║ ██╔██╗     ██╔═══╝ ██╔══██║   ██║   ██║     ██╔══██║    ██║███╗██║██╔══██║   ██║   ██║     ██╔══██║</Text>
        </Gradient>
      </Box>
      <Box>
        <Gradient name="rainbow">
          <Text bold>  ██║ ╚████║██║██╔╝ ██╗    ██║     ██║  ██║   ██║   ╚██████╗██║  ██║    ╚███╔███╔╝██║  ██║   ██║   ╚██████╗██║  ██║</Text>
        </Gradient>
      </Box>

      <Box marginTop={1} gap={2}>
        {tabs.map((tab, i) => (
          <Box key={tab} borderStyle={activeTab === i ? 'bold' : 'single'} borderColor={activeTab === i ? 'cyan' : 'gray'} paddingX={1}>
            <Text color={activeTab === i ? 'cyan' : 'gray'} bold={activeTab === i}>
              {activeTab === i ? '▸ ' : '  '}{tab} {activeTab === i ? '◂' : '  '}
            </Text>
          </Box>
        ))}
        <Box marginLeft="auto" gap={1}>
          <Text dimColor>[Tab] next  [M] submit MR  [R] refresh  [Q] quit</Text>
        </Box>
      </Box>
    </Box>
  );
}
