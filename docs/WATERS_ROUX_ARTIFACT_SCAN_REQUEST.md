# Exact Waters Roux Windows artifact security scan

This PR requests the non-executing Windows Defender scan of the original Integrated Beta 27 GitHub Actions artifact ID **13785467800**. Confirm the nested ZIP and executable have the pinned SHA-256 values, refresh antivirus definitions, scan the extracted package, and fail on detected threats. The requested scan does not execute the viewer or bypass browser protections. A clean Defender scan is limited evidence and does not override Google's Safe Browsing warning.
