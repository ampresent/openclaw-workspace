#!/bin/bash
LOG=/tmp/bootc-switch.log
echo "=== bootc switch start: $(date) ===" >> $LOG
bootc switch registry.fedoraproject.org/fedora-bootc:42 >> $LOG 2>&1
echo "=== exit: $? at $(date) ===" >> $LOG
bootc status >> $LOG 2>&1
