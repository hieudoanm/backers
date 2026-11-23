import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import type { FC, ReactNode } from 'react';
import Footer from '@/components/molecules/Footer';
import Navbar from '@/components/molecules/Navbar';
import './globals.css';

const inter = Inter({ subsets: ['latin'] });

const GITHUB = 'https://github.com/hieudoanm/backers';
const LOGO =
  process.env.NODE_ENV === 'production' ? '/backers/logo.svg' : '/logo.svg';
const NAV_LINKS = [
  { label: 'Features', href: '/#features' },
  { label: 'Versus', href: '/#compare' },
  { label: 'Quickstart', href: '/#quickstart' },
  { label: 'Docs', href: '/docs/' },
  { label: 'Downloads', href: '/downloads/' },
];

export const metadata: Metadata = {
  metadataBase: new URL('https://hieudoanm.github.io/backers/'),
  title: {
    default: 'Backers — Back-end as a Service',
    template: '%s — Backers',
  },
  description:
    'Auth, a schema-less database, file storage, realtime, webhooks, cron, secrets, pub/sub and an admin dashboard — in one Rust binary on a single SQLite file. Inspired by PocketBase and Supabase.',
  keywords: [
    'Back-end as a Service',
    'BaaS',
    'PocketBase alternative',
    'Supabase alternative',
    'SQLite',
    'Rust',
    'authentication',
    'file storage',
    'realtime',
    'webhooks',
    'cron jobs',
    'MCP server',
  ],
  authors: [{ name: 'hieudoan', url: 'https://github.com/hieudoanm/backers' }],
  category: 'software',
  openGraph: {
    siteName: 'Backers',
    type: 'website',
    title: 'Backers — Back-end as a Service',
    description:
      'Auth, database, files, realtime, webhooks, cron, secrets and an MCP server — one Rust binary, one SQLite file.',
    url: 'https://hieudoanm.github.io/backers/',
    images: ['/og.png'],
  },
  twitter: { card: 'summary_large_image', images: ['/og.png'] },
};

const JSON_LD = {
  '@context': 'https://schema.org',
  '@graph': [
    {
      '@type': 'WebSite',
      name: 'Backers',
      url: 'https://hieudoanm.github.io/backers/',
      description:
        'Auth, a schema-less database, file storage, realtime, webhooks, cron, secrets, pub/sub and an admin dashboard — in one Rust binary on a single SQLite file. Inspired by PocketBase and Supabase.',
    },
    {
      '@type': 'SoftwareApplication',
      name: 'Backers',
      applicationCategory: 'DeveloperApplication',
      description:
        'Auth, a schema-less database, file storage, realtime, webhooks, cron, secrets, pub/sub and an admin dashboard — in one Rust binary on a single SQLite file. Inspired by PocketBase and Supabase.',
      url: 'https://hieudoanm.github.io/backers/',
      codeRepository: 'https://github.com/hieudoanm/backers',
      offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
    },
  ],
};

interface RootLayoutProps {
  children: ReactNode;
}

const RootLayout: FC<RootLayoutProps> = ({ children }) => (
  <html
    lang="en"
    data-theme="backers"
    className={`scroll-smooth antialiased motion-reduce:scroll-auto ${inter.className}`}>
    <body className="bg-base-100 text-base-content m-0 leading-relaxed">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(JSON_LD) }}
      />
      <Navbar
        name="backers"
        logoSrc={LOGO}
        links={NAV_LINKS}
        githubUrl={GITHUB}
      />
      {children}
      <Footer
        name="backers"
        tagline="Back-end as a Service · Rust · SQLite · MIT"
        githubUrl={GITHUB}
      />
      
    </body>
  </html>
);

export default RootLayout;
