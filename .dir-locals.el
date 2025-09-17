;;; Update PATH from esp exports into local process environment
((nil .
      ;;; ((lsp-rust-analyzer-proc-macro-enable . nil))
      ((eval .
	     (progn
	       (make-local-variable 'process-environment)
	       (with-temp-buffer
		 (call-process "bash" nil t nil "-c"
			       "source ~/export-esp.sh; env | egrep '^PATH='")
		 (insert "\nSSID=testssid\n")
		 (insert "PASSWORD=testpassword\n")
		 (goto-char (point-min))
		 (while (not (eobp))
		   (setq process-environment
			 (cons (buffer-substring (point) (line-end-position))
			       process-environment))
		   (forward-line 1))))))))
