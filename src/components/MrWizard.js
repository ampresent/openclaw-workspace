import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import TextInput from 'ink-text-input';
import Spinner from 'ink-spinner';
import { execa } from 'execa';

export default function MrWizard({ item, onClose }) {
  const [step, setStep] = useState(0);
  const [branchName, setBranchName] = useState('');
  const [commitMsg, setCommitMsg] = useState('');
  const [targetRemote, setTargetRemote] = useState('origin');
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState(null);

  const steps = [
    { label: 'Branch name', placeholder: `patch/${item?.name?.toLowerCase().replace(/\s+/g, '-') || 'my-patch'}` },
    { label: 'Commit message', placeholder: item?.description || `Apply patch: ${item?.name || 'unnamed'}` },
    { label: 'Target remote', placeholder: 'origin' },
  ];

  useInput((input, key) => {
    if (submitting) return;

    if (key.escape || (key.ctrl && input === 'c')) {
      onClose();
    }
    if (key.return) {
      if (step < steps.length - 1) {
        setStep(step + 1);
      } else {
        handleSubmit();
      }
    }
  });

  async function handleSubmit() {
    setSubmitting(true);
    try {
      const branch = branchName || steps[0].placeholder;
      const msg = commitMsg || steps[1].placeholder;
      const remote = targetRemote || steps[2].placeholder;

      // Create branch
      await execa('git', ['checkout', '-b', branch]);
      // Stage changes
      await execa('git', ['add', '-A']);
      // Commit
      await execa('git', ['commit', '-m', msg]);
      // Push
      await execa('git', ['push', '-u', remote, branch]);

      setResult({ success: true, branch, msg });
    } catch (err) {
      setResult({ success: false, error: err.message });
    }
    setSubmitting(false);
  }

  if (result) {
    return (
      <Box
        position="absolute"
        width="70%"
        height="40%"
        borderStyle="double"
        borderColor={result.success ? 'green' : 'red'}
        flexDirection="column"
        padding={1}
      >
        <Text bold color={result.success ? 'green' : 'red'}>
          {result.success ? '✓ MR Branch Created!' : '✗ Failed'}
        </Text>
        {result.success ? (
          <Box flexDirection="column" marginTop={1}>
            <Text>Branch: <Text cyan>{result.branch}</Text></Text>
            <Text>Message: {result.msg}</Text>
            <Box marginTop={1}>
              <Text dimColor>
                Next: Go to your Git forge to create the merge request.
              </Text>
            </Box>
          </Box>
        ) : (
          <Text color="red">{result.error}</Text>
        )}
        <Box marginTop={1}>
          <Text dimColor>Press Esc to close</Text>
        </Box>
      </Box>
    );
  }

  return (
    <Box
      position="absolute"
      width="60%"
      height="50%"
      borderStyle="double"
      borderColor="magenta"
      flexDirection="column"
      padding={1}
    >
      <Text bold color="magenta">
        {submitting ? <Spinner type="dots" /> : '🦊'} Submit Merge Request
      </Text>
      <Box marginTop={1} flexDirection="column" gap={1}>
        {steps.map((s, i) => (
          <Box key={s.label} flexDirection="column">
            <Box>
              <Text color={i < step ? 'green' : i === step ? 'cyan' : 'gray'}>
                {i < step ? '✓ ' : i === step ? '▸ ' : '  '}
                {s.label}:
              </Text>
            </Box>
            {i === step && !submitting && (
              <Box paddingLeft={4}>
                <TextInput
                  value={
                    i === 0 ? branchName :
                    i === 1 ? commitMsg :
                    targetRemote
                  }
                  onChange={
                    i === 0 ? setBranchName :
                    i === 1 ? setCommitMsg :
                    setTargetRemote
                  }
                  placeholder={s.placeholder}
                />
              </Box>
            )}
            {i < step && (
              <Box paddingLeft={4}>
                <Text dimColor>
                  {i === 0 ? (branchName || s.placeholder) :
                   i === 1 ? (commitMsg || s.placeholder) :
                   (targetRemote || s.placeholder)}
                </Text>
              </Box>
            )}
          </Box>
        ))}
      </Box>
      <Box marginTop={1}>
        <Text dimColor>[Enter] next/submit  [Esc] cancel</Text>
      </Box>
    </Box>
  );
}
