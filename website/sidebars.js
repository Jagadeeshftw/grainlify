/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  docsSidebar: [
    'introduction',
    {
      type: 'category',
      label: 'Core info',
      collapsed: false,
      items: [
        'what-is-grainlify',
        'why-grainlify',
        'how-it-works',
      ],
    },
    {
      type: 'category',
      label: 'Key concepts',
      collapsed: false,
      items: [
        'user-roles',
        'programs-and-bounties',
      ],
    },
    {
      type: 'category',
      label: 'User guides',
      collapsed: false,
      items: [
        'for-contributors',
        'for-maintainers',
        'for-ecosystems',
        'profile-page',
      ],
    },
    {
      type: 'category',
      label: 'Technical',
      collapsed: true,
      items: [
        'product-overview',
        'architecture',
        'smart-contracts',
        'security-and-compliance',
      ],
    },
    {
      type: 'category',
      label: 'Additional info',
      collapsed: true,
      items: [
        'community-and-support',
        'kyc-privacy-notice',
        'terms-and-conditions',
      ],
    },
  ],
};

export default sidebars;
