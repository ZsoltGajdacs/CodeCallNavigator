import {useCallback, useEffect} from 'react';

export function leaf() { return 1; }
export function disconnected() { return 2; }
function Child({onGo}: {onGo: () => number}) {
  return <button onClick={onGo}>Run</button>;
}
export default function Page() {
  const callback = useCallback(leaf, []);
  useEffect(() => { leaf(); return () => { leaf(); }; }, []);
  return <Child onGo={callback}/>;
}
