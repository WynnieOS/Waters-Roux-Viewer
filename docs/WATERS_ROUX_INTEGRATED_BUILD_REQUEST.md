# Integrated Waters Roux Beta 27 verification request

This branch triggers the isolated Windows integration workflow. The workflow checks out the Beta 27 branded feature branch, merges the toolbar/360 and rotation-edit branches **inside the CI runner only**, validates the merged tree, and packages a new downloadable Windows ZIP only on successful completion. It does not merge the feature branches into main.

Required acceptance: all branch merges succeed, Windows format/lint/tests/release compilation succeed, ZIP artifact uploaded, exact integrated commit tracked, and owner tests login, world selection/rendering and photo tools in Second Life. No claim that unfinished avatar tagging, recording or poser features are included.
