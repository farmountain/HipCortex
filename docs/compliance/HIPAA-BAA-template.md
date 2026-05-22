# Business Associate Agreement (BAA) Template

**This is a template only. Have your legal counsel review before signing.**

---

## BUSINESS ASSOCIATE AGREEMENT

This Business Associate Agreement ("BAA") is entered into between:

**Covered Entity:** [CUSTOMER NAME], a [STATE] [ENTITY TYPE] ("Covered Entity")

**Business Associate:** [YOUR COMPANY NAME] ("Business Associate"), the operator of HipCortex Memory Engine

---

## RECITALS

WHEREAS, Business Associate provides AI memory engine services to Covered Entity through the HipCortex platform, which may involve the creation, receipt, maintenance, or transmission of Protected Health Information ("PHI") as defined by HIPAA; and

WHEREAS, the parties wish to ensure compliance with HIPAA and the HITECH Act;

NOW, THEREFORE, the parties agree as follows:

---

## ARTICLE 1: DEFINITIONS

**"PHI"** means Protected Health Information as defined in 45 CFR § 160.103.

**"HIPAA Rules"** means the HIPAA Privacy Rule (45 CFR Part 164) and Security Rule.

**"Services"** means AI memory engine services including persistent memory storage, retrieval, and analysis provided through HipCortex.

---

## ARTICLE 2: OBLIGATIONS OF BUSINESS ASSOCIATE

2.1 **Use and Disclosure Limitations.** Business Associate shall not use or disclose PHI other than as permitted by this BAA or required by law.

2.2 **Safeguards.** Business Associate shall implement appropriate administrative, physical, and technical safeguards to protect PHI, including:
- AES-256-GCM encryption of PHI at rest
- TLS 1.3 encryption in transit
- Merkle-chained audit logs for all PHI access
- GDPR-compliant right-to-erasure (DELETE /memory/forget/:actor)
- Role-based access control via API key tiers

2.3 **Subcontractors.** Business Associate shall ensure any subcontractors agree to the same restrictions.

2.4 **Breach Notification.** Business Associate shall notify Covered Entity of any breach of unsecured PHI within [60] days.

2.5 **Access.** Business Associate shall provide PHI to the Covered Entity upon request.

2.6 **Minimum Necessary.** Business Associate shall use or disclose only the minimum necessary PHI.

---

## ARTICLE 3: TERM AND TERMINATION

3.1 This BAA shall remain in effect as long as Business Associate maintains PHI.

3.2 Upon termination, Business Associate shall return or destroy all PHI.

---

## ARTICLE 4: PERMITTED USES

Business Associate may use PHI to:
- Provide the contracted AI memory services
- Perform data aggregation for permitted purposes
- De-identify information per 45 CFR § 164.514

---

## ARTICLE 5: SIGNATURES

**Covered Entity:**
Signature: ___________________________
Name: ________________________________
Title: _______________________________
Date: ________________________________

**Business Associate:**
Signature: ___________________________
Name: ________________________________
Title: _______________________________
Date: ________________________________

---

*For BAA inquiries: hipcortex@farmountain.dev*
*Subject: BAA Request — [Your Organization]*
