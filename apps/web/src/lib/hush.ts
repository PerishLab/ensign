import { useEffect } from "react";

export function useHush(clear: () => void): void {
	useEffect(() => {
		const drop = () => clear();
		globalThis.addEventListener?.("pagehide", drop);
		return () => globalThis.removeEventListener?.("pagehide", drop);
	}, [clear]);
}
