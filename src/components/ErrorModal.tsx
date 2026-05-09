import { useEffect } from "react";

interface ErrorModalProps {
	message: string | null;
	onDismiss: () => void;
	/// Optional title; defaults to a soft, non-clinical phrasing.
	title?: string;
}

/// Centered overlay modal for surfacing errors that the user needs to see
/// and acknowledge. Renders nothing when `message` is null. Keyboard:
/// Escape dismisses; backdrop click dismisses.
export function ErrorModal({ message, onDismiss, title }: ErrorModalProps) {
	useEffect(() => {
		if (message === null) return;
		const handleKey = (event: KeyboardEvent) => {
			if (event.key === "Escape") onDismiss();
		};
		window.addEventListener("keydown", handleKey);
		return () => window.removeEventListener("keydown", handleKey);
	}, [message, onDismiss]);

	if (message === null) return null;

	return (
		<button
			type="button"
			className="error-modal-backdrop"
			onClick={onDismiss}
			aria-label="Dismiss error"
		>
			<div
				className="error-modal-card"
				role="alertdialog"
				aria-modal="true"
				aria-labelledby="error-modal-title"
				onClick={(event) => event.stopPropagation()}
				onKeyDown={(event) => event.stopPropagation()}
			>
				<div className="error-modal-icon" aria-hidden="true">
					🐾
				</div>
				<h2 id="error-modal-title" className="error-modal-title">
					{title ?? "Something went sideways."}
				</h2>
				<p className="error-modal-message">{message}</p>
				<button
					type="button"
					className="primary error-modal-dismiss"
					onClick={onDismiss}
				>
					Got it
				</button>
			</div>
		</button>
	);
}
