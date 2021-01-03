#!/bin/zsh
#/home/harddisk/arek/projects/_patching/fs-uae-3.0.5/fs-uae /home/harddisk/arek/documents/FS-UAE/Configurations/Ambermoon.fs-uae > /tmp/opcode.log
/home/harddisk/arek/projects/_patching/fs-uae-3.0.5/fs-uae /home/harddisk/arek/documents/FS-UAE/Configurations/EAmbermoon.fs-uae > /tmp/opcode.log

echo "description: (empty for dont copy)"
read desc
if [ "$desc" = "" ] ; then
	exit 0
fi
mkdir $desc
mv /tmp/opcode.log $desc
cp offset $desc/
#cd $desc
#/home/harddisk/arek/amiga/ambm/dump-analyzers/extract_start_pcs.py .
