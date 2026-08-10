const http = require('http');
http.get('http://localhost:3000/admin/predicates', (res) => {
  let data = '';
  res.on('data', c => data += c);
  res.on('end', () => {
    // Look for error message in HTML
    const idx = data.indexOf('500');
    if (idx > -1) {
      console.log('Context around 500:', data.substring(Math.max(0, idx - 100), idx + 200));
    }
    // Look for __NUXT__ data
    const nuxtIdx = data.indexOf('__NUXT__');
    if (nuxtIdx > -1) {
      console.log('NUXT data (first 500):', data.substring(nuxtIdx, nuxtIdx + 500));
    }
  });
});
