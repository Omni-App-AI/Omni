import requests, json, re
url='https://jobs.mchire.com/jobs?location=29630'
text=requests.get(url, timeout=30).text
m=re.search(r'window.__PRELOAD_STATE__\s*=\s*(\{.*?\})\s*function OptanonWrapper', text, re.S)
print('found', bool(m))
if m:
    data=json.loads(m.group(1))
    jobs=data['jobSearch']['jobs']
    matches=[]
    for j in jobs:
        for loc in j.get('locations',[]):
            city=(loc.get('city') or '').upper()
            if loc.get('zipCode')=='29630' or loc.get('postalCode')=='29630' or city=='CLEMSON':
                matches.append({
                    'title': j.get('title'),
                    'category': ', '.join(c.get('name','') for c in j.get('categories',[])),
                    'location': loc.get('locationText'),
                    'applyURL': j.get('applyURL')
                })
    print(json.dumps(matches, indent=2))
