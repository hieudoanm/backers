import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'Backers Demo',
  description:
    'Live demo of the Backers BaaS — auth, records, realtime, files.',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" data-theme="backers">
      <body className="bg-base-200 text-base-content min-h-screen">
        {children}
      </body>
    </html>
  );
}
