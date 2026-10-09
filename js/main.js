/**
 * Lightweight vanilla JavaScript helper for Wael El Oraiby's portfolio
 * - Theme toggle (Light/Dark mode) with persistent storage
 * - Mobile navigation menu
 * - Blog category/tag filtering
 */

(function () {
  'use strict';

  // --- 1. Theme Management ---
  const THEME_KEY = 'portfolio-theme';
  const root = document.documentElement;

  function getPreferredTheme() {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved) return saved;
    // System preference fallback (defaulting to dark if unsure)
    return window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches
      ? 'light'
      : 'dark';
  }

  function setTheme(theme) {
    root.setAttribute('data-theme', theme);
    localStorage.setItem(THEME_KEY, theme);
  }

  // Apply initially before DOM fully loads to prevent flash of wrong theme
  setTheme(getPreferredTheme());

  document.addEventListener('DOMContentLoaded', () => {
    // Theme toggle buttons
    const toggleBtns = document.querySelectorAll('.theme-toggle-btn');
    toggleBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        const current = root.getAttribute('data-theme') || 'dark';
        const next = current === 'dark' ? 'light' : 'dark';
        setTheme(next);
      });
    });

    // Listen for OS system theme changes if user hasn't explicitly chosen
    if (window.matchMedia) {
      window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        if (!localStorage.getItem(THEME_KEY)) {
          setTheme(e.matches ? 'dark' : 'light');
        }
      });
    }

    // --- 2. Mobile Menu Toggle ---
    const mobileBtn = document.querySelector('.mobile-menu-btn');
    const navMenu = document.querySelector('.nav-menu');
    if (mobileBtn && navMenu) {
      mobileBtn.addEventListener('click', () => {
        const isOpen = navMenu.classList.toggle('open');
        mobileBtn.setAttribute('aria-expanded', isOpen);
      });

      // Close menu when clicking outside or navigating
      document.addEventListener('click', (e) => {
        if (!mobileBtn.contains(e.target) && !navMenu.contains(e.target)) {
          navMenu.classList.remove('open');
          mobileBtn.setAttribute('aria-expanded', 'false');
        }
      });
    }

    // --- 3. Blog Tag Filtering (Blog Index) ---
    const filterBtns = document.querySelectorAll('.filter-btn');
    const blogCards = document.querySelectorAll('.blog-entry-card');

    if (filterBtns.length > 0 && blogCards.length > 0) {
      filterBtns.forEach((btn) => {
        btn.addEventListener('click', () => {
          const filter = btn.getAttribute('data-filter');

          // Update active button state
          filterBtns.forEach((b) => b.classList.remove('active'));
          btn.classList.add('active');

          // Filter cards
          blogCards.forEach((card) => {
            const tags = (card.getAttribute('data-tags') || '').toLowerCase().split(',');
            if (filter === 'all' || tags.includes(filter.toLowerCase())) {
              card.style.display = 'flex';
            } else {
              card.style.display = 'none';
            }
          });
        });
      });
    }
  });
})();

