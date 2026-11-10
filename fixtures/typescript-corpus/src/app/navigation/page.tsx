'use client';
import Link from 'next/link';
import {useRouter} from 'next/navigation';

export default function NavigationPage() {
  const router = useRouter();
  function openAbout() { router.push('/about'); }
  function replaceUser() { router.replace('/users/42'); }
  function prefetchOnly() { router.prefetch('/about'); }
  return <><Link href="/about">About</Link><button onClick={openAbout}>About</button>
    <button onClick={replaceUser}>User</button><button onClick={prefetchOnly}>Warm</button></>;
}
