import { h, type App } from 'vue';
import { useData } from 'vitepress';
import DefaultTheme from 'vitepress/theme';
import HamrLanding from './components/HamrLanding.vue';
import './style.css';

export default {
  extends: DefaultTheme,
  Layout() {
    const { frontmatter } = useData();
    if (frontmatter.value.layout === 'hamr-landing') {
      return h(HamrLanding);
    }
    return h(DefaultTheme.Layout, null, {
      'nav-bar-title-before': () =>
        h('div', { class: 'hamr-nav-logo' }, [
          h('span', { class: 'hamr-nav-mark' }, '⚒'),
          h('span', { class: 'hamr-nav-text' }, 'hamr'),
        ]),
    });
  },
  enhanceApp({ app }: { app: App }) {
    app.component('HamrLanding', HamrLanding);
  },
};
