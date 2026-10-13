//! Port of source/world/generator/newsgenerator.h + source/world/generator/newsgenerator.cpp

use crate::ctx::Ctx;
use crate::world::company::companyuplink::CompanyUplink;
use crate::world::company::news::{
    News, NEWS_TYPE_ARREST, NEWS_TYPE_COMPUTERDESTROYED, NEWS_TYPE_COMPUTERSHUTDOWN,
    NEWS_TYPE_DELETED, NEWS_TYPE_HACKED, NEWS_TYPE_STOLEN,
};
use crate::world::computer::computer::COMPUTER_TYPE_PUBLICBANKSERVER;
use crate::world::computer::logbank::AccessLog;
use crate::world::generator::numbergenerator::NumberGenerator;

/// Receives information on important events (e.g. a company going bust) and generates news
/// stories. Stateless: all functions are associated fns; computers and people are passed by key.
pub struct NewsGenerator;

/// The Computer fields the news texts use, copied out so the world can be borrowed mutably
/// (for `CompanyUplink::create_news`) afterwards.
struct CompInfo {
    name: String,
    companyname: String,
    ip: String,
    type_: i32,
    recent_hacks: i32,
}

fn comp_info(g: &Ctx, comp: &str) -> CompInfo {
    let c = g.world().get_computer(comp).expect("UplinkAssert(comp)");
    CompInfo {
        name: c.name.clone(),
        companyname: c.companyname.clone(),
        ip: c.ip.clone(),
        type_: c.type_,
        recent_hacks: c.num_recent_hacks(),
    }
}

/// C++ `part1 << "\n\n" << part2 << "\n\n" ...` concatenation.
fn join_parts(parts: &[&str]) -> String {
    parts.join("\n\n")
}

/// C++ `News *news = new News(); SetHeadline; SetDetails; SetData; cu->CreateNews(news);`
fn post_news(
    g: &mut Ctx,
    headline: &str,
    details: &str,
    newstype: i32,
    data1: &str,
    data2: Option<&str>,
) {
    let mut news = News::new();
    news.set_headline(headline);
    news.set_details(details);
    news.set_data(newstype, Some(data1), data2);
    CompanyUplink::create_news(g, news);
}

impl NewsGenerator {
    pub fn initialise() {}

    // Events related to Company computers

    /// Hacked by unknown person. `comp` = Computer name.
    pub fn computer_hacked(g: &mut Ctx, comp: &str, al: &AccessLog) {
        /* ==========================================================
            Structure:

            Headline
            Part1 : Basic intro (eg ss databased hacked again)
            Part2 : Recent number of hacks
            [part3 : Date of attack]  ( automatic )
            part4 : Future or existing effects of hack
            part5 : Promised action
            part6 : Misc

           ========================================================== */

        let ci = comp_info(g, comp);
        let mut headline = String::new();
        let mut part1 = String::new();
        let mut part2 = String::new();
        let mut part3 = String::new();
        let mut part4 = String::new();
        let mut part5 = String::new();
        let part6 = String::new();

        // ================================================================================================================
        // Social security database
        // ================================================================================================================

        if ci.name == "International Social Security Database" {
            match ci.recent_hacks {
                0 | 1 => {
                    headline.push_str("Government social security records compromised");
                    part1.push_str("The Government run social security database has been hacked by an unknown cyber criminal.");
                    part2.push_str("Government officials insist this is the first and only incident of its kind.");
                    part4.push_str(concat!(
                        "In theory, unlawful changes to this database could result in dead people being reported as alive, ",
                        "or alive people being reported as dead.  Other personal information could be stolen."
                    ));
                    part5.push_str("A government federal agent today stated that this would not be allowed to happen again.");
                }

                2 => {
                    headline.push_str("Social security hacked twice in less than a month");
                    part1.push_str("The Government run social security database has once again been hacked by an unknown cyber criminal.  ");
                    part2.push_str(concat!(
                        "This is the second time in less than a month that this system has been attacked.  ",
                        "It is not known if these two incidents are connected."
                    ));
                    part4.push_str("A hacker gaining access to this system could do untold damage by modifying crucial personal data.");
                    part5.push_str("The government today launched an investigation into these recent security problems.");
                }

                3 => {
                    headline.push_str("Government held Files compromised for the third time");
                    part1.push_str("For the third time in less than a month, the government owned Social Security Database has been hacked.");
                    part4.push_str(concat!(
                        "The system holds key personal data on millions of people and has been the cause of some embarrasment recently ",
                        "for the government."
                    ));
                    part5.push_str(concat!(
                        "The potential for damage on this system is huge, prompting calls by opposition parties for a full ",
                        "review of all security protocols."
                    ));
                }

                _ => {
                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            headline.push_str("Government Social Security files accessed again");
                            part4.push_str(concat!(
                                "The system holds personal data on the general population, but could be maliciously used to create ",
                                "new identities or tamper with existing ones."
                            ));
                        }
                        2 => {
                            headline.push_str("Social Security computer once again hacked");
                            part4.push_str(concat!(
                                "It is feared that the many recent hacks of this system may lead to a lack of trust in its data, which ",
                                "is used all over industry for many purposes."
                            ));
                        }
                        3 => {
                            headline.push_str("Government maintained Social Security documents come under attack again");
                            part4.push_str(concat!(
                                "In theory, a skilled hacker could manufacture a new identity, or tamper with an existing record ",
                                "to cause damage to a person's records.  "
                            ));
                        }
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => part2.push_str("This system has been hacked multiple times in the last month, prompting heavy critisism from some opposition parties."),
                        2 => part2.push_str("This is not the first time this has happened; this system has been tampered with more than once recently."),
                        3 => part2.push_str("The Government is once again on the defensive as this system seems to be almost wide open to abuse."),
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            part1.push_str("Government social security documents have been accessed once again.");
                            part5.push_str("Government spokesmen today insisted they had the situation under control, despite recent history.");
                        }
                        2 => {
                            part1.push_str("Government maintained Social Security documents have come under attack yet again.");
                            part5.push_str("The government is said to be considering many options on how to stop these attacks.");
                        }
                        3 => {
                            part1.push_str("The Goverment owned Social Security computer has once again been hacked by an unknown Cyber criminal.");
                            part5.push_str("Federal agents have been called in to help catch the criminals responsible for this action.");
                        }
                        _ => {}
                    }
                }
            }
        }

        // ================================================================================================================
        // Academic database
        // ================================================================================================================

        else if ci.name == "International Academic Database" {
            match ci.recent_hacks {
                0 | 1 => {
                    headline.push_str("Government run academic database hacked");
                    part1.push_str("The International Academic Database has been compromised by  malicious hacker.");
                    part2.push_str("Until today this system was thought to be secure against this type of attack.");
                    part4.push_str(concat!(
                        "The motives of the hacker are not clear but it is thought he was attempting to forge ",
                        "qualifications for an unknown recipient."
                    ));
                    part5.push_str(concat!(
                        "Government agents made it clear they would not tolerate this action and have already taken steps ",
                        "to secure the database against future action."
                    ));
                }

                2 => {
                    headline.push_str("Academic records hacked again");
                    part1.push_str("The International Academic Database has been hacked for the second time in less than a month.");
                    part4.push_str("It is not known if the two attacks are connected.");
                    part5.push_str("The government today began an investigation into these attacks.");
                }

                3 => {
                    headline.push_str("Government academic database hacked again");
                    part1.push_str("The Government is on the defensive again today as its International Academic Database is hacked again.");
                    part2.push_str("This attack comes less than a week after a recent hack and is the third this month.");
                    part4.push_str(concat!(
                        "It is believed that an individual or small group is attempting to create new academic details and qualifications, ",
                        "perhaps offering this as a service."
                    ));
                    part5.push_str("Federal agents have been brought in to investigate.");
                }

                _ => {
                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            headline.push_str("Government embarresed over repeated security failures");
                            part5.push_str("The Government has again promised to put a stop to these hacks quickly.");
                        }
                        2 => {
                            headline.push_str("International Academic Database hacked again");
                            part5.push_str("The Government seems to be at a loss as to what to do about these recent attacks.");
                        }
                        3 => {
                            headline.push_str("Federal Agents investigate Academic Database hacks");
                            part5.push_str(concat!(
                                "Federal Agents from the computer crimes division where today tasked with catching the criminals involved ",
                                "and determining what damage has been done."
                            ));
                        }
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => part2.push_str("Security on this computer has been bypassed repeadly in the last month."),
                        2 => part2.push_str("This attack is the latest in a string of hacks performed in the last month."),
                        3 => part2.push_str("Federal agents seem unable to stop a torrent of recent hacks on this system."),
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            part1.push_str("The Government Academic Database has been hacked again by an unknown intruder.");
                            part4.push_str("The system is used to store the Academic history of millions of people.");
                        }
                        2 => {
                            part1.push_str("Hackers have again penetrated the International Acadmic Database.");
                            part4.push_str("In theory a skilled hacker could create new qualifications for any person he wished.");
                        }
                        3 => {
                            part1.push_str("The International Academic Database has been attacked by hackers again.");
                            part4.push_str(concat!(
                                "The exact motives of the hackers are not clear, but it is believed they may be running ",
                                "some kind of buisiness offering new qualifications for people for a price."
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }

        // ================================================================================================================
        // Global Criminal Database
        // ================================================================================================================

        else if ci.name == "Global Criminal Database" {
            match ci.recent_hacks {
                0 | 1 => {
                    headline.push_str("Global Criminal Records attacked by hackers");
                    part1.push_str("The Government managed Global Criminal Database has been penetrated by an unknown computer hacker.");
                    part2.push_str("Security on the system is so tight that hacks of this kind were thought to be impossible.");
                    part4.push_str(concat!(
                        "It appears from a preliminary investigation that an unknown criminal record was modified, prompting ",
                        "calls from human rights groups for higher security on personal records of this kind."
                    ));
                    part5.push_str("The Government has said they have begun an investigation and have taken steps to stop this from happening again.");
                }

                2 => {
                    headline.push_str("Criminal Records opened to the public domain");
                    part1.push_str(concat!(
                        "Despite the installation of a brand new high security system, the Global Criminal Database has again been attacked ",
                        "and penetrated by a hacker or group of hackers."
                    ));
                    part2.push_str("Government agents are said to be amazed that their new system has failed so soon.");
                    part4.push_str("This is the second time criminal records have been modified by hackers masquerading as officers.");
                    part5.push_str("The newly installed system will no doubt be tested in light of this new breach.");
                }

                3 => {
                    headline.push_str("Hackers take advantage of Global Criminal Database");
                    part1.push_str(concat!(
                        "The Government owned Global Criminal Database system has recently suffered a couple of high security hacks, ",
                        "and it would seem more and more hackers are taking advantage of the system."
                    ));
                    part2.push_str(concat!(
                        "For the third time in less than a month a hacker gained adminstrative access to the system and modified ",
                        "criminal records within the system."
                    ));
                    part4.push_str(concat!(
                        "The Government now believes a group of individuals are offering modifications to personal records such as criminal records ",
                        "for a price, perhaps advertising through the underground anarchist group Uplink Corporation."
                    ));
                    part5.push_str("The Government has now clamped down on accesses to the system, claiming it is safer than ever.");
                }

                _ => {
                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            headline.push_str("Global Criminal Database smashed open again");
                            part5.push_str("The Government appears to be unable to put a stop to these attacks.");
                        }
                        2 => {
                            headline.push_str("Government criminal records system hacked yet again");
                            part5.push_str("The Government appear to be at a loss as to what to do.");
                        }
                        3 => {
                            headline.push_str("Criminal Records modified by hackers again");
                            part5.push_str("The Government is now considering shutting down the Global Criminal Database until this can be resolved.");
                        }
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => part2.push_str("Security at this server system has been compromised numerous times in the past month."),
                        2 => part2.push_str("This news service has unfortunately lost count of the number of hacks at this site in recent times."),
                        3 => part2.push_str("The system appears to have more security holes than popular 'Micro software' written in the late 20th century."),
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            part1.push_str("The Government run Global Criminal Database has again been hacked wide open.");
                            part4.push_str("It is believed an underground organisation is selling the service of modifying records.");
                        }
                        2 => {
                            part1.push_str("The Global Criminal Database has been successfully attacked once again.");
                            part4.push_str("Hackers could clear existing records or could have innocent people thrown in jail.");
                        }
                        3 => {
                            part1.push_str("Once again the Global Criminal Database has been hacked.");
                            part4.push_str("A number of cases have now arisen of people in prison who claim they were framed in this way.");
                        }
                        _ => {}
                    }
                }
            }
        }

        // ================================================================================================================
        // Bank Server
        // ================================================================================================================

        else if ci.type_ == COMPUTER_TYPE_PUBLICBANKSERVER {
            match ci.recent_hacks {
                0 | 1 => {
                    headline.push_str(&format!("{} hacked", ci.name));
                    part1.push_str(&format!(
                        "The financial computer system {} has been penetrated from an unknown source.",
                        ci.name
                    ));
                    part2.push_str(&format!(
                        "This is the first incident of its kind for the owner, {}.",
                        ci.companyname
                    ));
                    part4.push_str("Hackers gaining access to this system could perform any number of fund transfers.");
                    part5.push_str(&format!("{} are investigating the incident.", ci.companyname));
                }

                2 => {
                    headline.push_str(&format!("{} comes under attack again", ci.name));
                    part1.push_str(&format!(
                        "The {} computer system has been hacked for the second time.",
                        ci.name
                    ));
                    part4.push_str("It is believed that some unauthorised money transfers were performed by the hackers.");
                    part5.push_str(concat!(
                        "This incident has lead to a fall in investor trust, and  the Bank is currently searching for ",
                        "clues as to the identity of the hackers responsible."
                    ));
                }

                3 => {
                    headline.push_str(&format!("{} hacked for the third time", ci.name));
                    part1.push_str(&format!(
                        "{} are at a loss today as their online trading system, {} was hacked for the third time in less than a month.",
                        ci.companyname, ci.name
                    ));
                    part4.push_str(&format!(
                        "Funds are being illegally transfered but {} are unable to provide any further details at this time.",
                        ci.companyname
                    ));
                    part5.push_str(concat!(
                        "Earlier today they issued a statement, saying that Federal Agents had been brought in ",
                        "to try to catch these hackers before they cause serious damage."
                    ));
                }

                _ => {
                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            headline.push_str(&format!("{} is hacked again", ci.name));
                            part2.push_str("This system has been hacked at least 4 times in the last month.");
                        }
                        2 => {
                            headline.push_str(&format!("{} security fails once again", ci.name));
                            part2.push_str(&format!(
                                "This is the latest in a long list of security failures for {}.",
                                ci.companyname
                            ));
                        }
                        3 => {
                            headline.push_str(&format!("{} is penetrated by hackers again", ci.name));
                            part2.push_str("Investor confidence is low as this system has been hacked many times recently.");
                        }
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => part1.push_str(&format!(
                            "The {} has been attacked and penetrated again by unknown cyber-criminals.",
                            ci.name
                        )),
                        2 => part1.push_str(&format!(
                            "Earlier today the {} system was attacked again by Computer hackers.",
                            ci.name
                        )),
                        3 => part1.push_str(&format!(
                            "{} suffered another embarrasing security problem today as their {} system was hacked from an unknown source.",
                            ci.companyname, ci.name
                        )),
                        _ => {}
                    }

                    match NumberGenerator::random_number(3) + 1 {
                        1 => {
                            part4.push_str("It is believed the hackers were attempting to transfer funds without authorisation.");
                            part5.push_str(&format!(
                                "{} have hired freelance computer specialists in an attempt to catch the guilty parties.",
                                ci.companyname
                            ));
                        }
                        2 => {
                            part4.push_str(concat!(
                                "Records indicate the hackers performed some kind of financial transfer, then attempted to remove ",
                                "any traces of their actions."
                            ));
                            part5.push_str(&format!(
                                "{} say they are are confident they will catch the hacker.",
                                ci.companyname
                            ));
                        }
                        3 => {
                            part4.push_str("Hackers gaining access to this system could perform any number of fund transfers between accounts.");
                            part5.push_str(&format!(
                                "{} are said to be considering further action in light of the number of recent hacks.",
                                ci.companyname
                            ));
                        }
                        _ => {}
                    }
                }
            }
        } else {
            // Not worth writing a story about it
            return;
        }

        //
        // Concatenate each part together and post it
        //

        part3.push_str(&format!("This occured on {}", al.date.get_long_string()));

        let details = join_parts(&[&part1, &part2, &part3, &part4, &part5, &part6]);
        post_news(g, &headline, &details, NEWS_TYPE_HACKED, &ci.ip, None);
    }

    pub fn computer_destroyed(g: &mut Ctx, comp: &str, filesdeleted: bool) {
        /* ==========================================================
            Structure

            headline
            Part1		:		Who was broken into, everything destroyed
            Part2		:		Details of destruction
            part2b		:		(If applicable) Files recovered
            Part3		:		Consequences
            Part4		:		Actions

           ========================================================== */

        let ci = comp_info(g, comp);
        let mut headline = String::new();
        let mut part1 = String::new();
        let mut part2 = String::new();
        let mut part3 = String::new();
        let mut part4 = String::new();

        match NumberGenerator::random_number(3) + 1 {
            1 => headline.push_str("Hackers take down major computer system"),
            2 => headline.push_str("Large corporation stung by malicious hacker attack"),
            3 => headline.push_str("Agressive computer attacker leaves system crippled"),
            _ => {}
        }

        match NumberGenerator::random_number(3) + 1 {
            1 => part1.push_str(&format!(
                "{} have taken a major blow to their operations today as a malicious computer hacker penetrated one of their key computer systems and caused terminal damage.",
                ci.companyname
            )),
            2 => part1.push_str(&format!(
                "A lone hacker has penetrated one of the primary computer systems of {} and deliberately shut down the system.",
                ci.companyname
            )),
            3 => part1.push_str(&format!(
                "Federal agents are now searching for the identity of a computer criminal who broke into the {} and deliberately destroyed the machine.",
                ci.name
            )),
            _ => {}
        }

        match NumberGenerator::random_number(3) + 1 {
            1 => part2.push_str(concat!(
                "It has been difficult to piece together exactly what happened, but it appears ",
                "that a system wide failure of the computer system was caused by the hackers actions.  ",
                "Experts say the entire operating system will have to be replaced."
            )),
            2 => part2.push_str(concat!(
                "Industry experts have re-created the final moments of the system crash from ",
                "access logs on another machine.  A few moments before the computer shut down, a lone ",
                "hacker forced his way into the system after bypassing the security, entered into ",
                "administrative mode and deleted several key system files, causing the machine to malfunction."
            )),
            3 => part2.push_str(concat!(
                "A single hacker is responsible for this high-tech crime.  Moments before the failure ",
                "he hacked into the system and deliberatly brought about its destruction.  The exact method has ",
                "not yet been determined."
            )),
            _ => {}
        }

        if filesdeleted {
            part2.push_str("\nThe company stated in their press release that all files had been totally destroyed.");
        } else {
            part2.push_str("\nExtensive damage was done but the company has been able to salvage most of the data.");
        }

        match NumberGenerator::random_number(3) + 1 {
            1 => part3.push_str(&format!(
                "The city witnessed a sharp drop in the share price today as news of this disaster reached investors.  It seems that nobody is safe after such a high profile system is destroyed.  {} said that millions of credits of damage had been done already.",
                ci.companyname
            )),
            2 => part3.push_str(concat!(
                "This system failure will cost the company dearly.  It seems the data on the system is ",
                "lost for good, and the costs of repairing this damage could spiral into millions of dollars.  ",
                "Industry analysts say the company is looking at some rough times ahead."
            )),
            3 => part3.push_str(concat!(
                "Today the company chairman tried to play down the significance of this event, no doubt ",
                "trying to avoid a public relations disaster as well as a system failure.  There can be little ",
                "doubt that this set back will cost the company dearly."
            )),
            _ => {}
        }

        match NumberGenerator::random_number(3) + 1 {
            1 => part4.push_str(concat!(
                "Company executives today released a press statement, indicating they were investigating ",
                "the incident and would be seeking outside assistance from industry professionals."
            )),
            2 => part4.push_str(concat!(
                "Federal agents have been brought in on the case and will no doubt be searching for ",
                "the identity of this dangerous computer hacker."
            )),
            3 => part4.push_str(&format!(
                "{} have tightened security on their remaining computer systems, and federal agents from the government cyber-crimes division were seen entering the corporate headquarters today.",
                ci.companyname
            )),
            _ => {}
        }

        //
        // Concatenate each part together and post it
        //

        let details = join_parts(&[&part1, &part2, &part3, &part4]);
        let newstype = if filesdeleted {
            NEWS_TYPE_COMPUTERDESTROYED
        } else {
            NEWS_TYPE_COMPUTERSHUTDOWN
        };
        post_news(g, &headline, &details, newstype, &ci.ip, None);
    }

    /// `filetype` is SCIENTIFIC, CORPORATE, CUSTOMER or SOFTWARE.
    pub fn all_files_stolen(g: &mut Ctx, comp: &str, filetype: &str, totalfilesize: i32) {
        /* =======================================================
            Structure:

            Headline
            Part1		:		Who was broken into, all files stolen
            Part2		:		Possible use of computer / type of data / Total size
            Part3		:		Consequences / Actions

           ====================================================== */

        let ci = comp_info(g, comp);
        let mut headline = String::new();
        let mut part1 = String::new();
        let mut part2 = String::new();
        let mut part3 = String::new();

        let totalfilesize = 10 * (totalfilesize / 10);

        match ci.recent_hacks {
            0 | 1 => part1.push_str(&format!(
                "{} discovered this morning that a massive quantity of computer data had been stolen from one of their Central Mainframes.",
                ci.companyname
            )),
            2 => part1.push_str(&format!(
                "A computer hacker entered the Central File Server of {} today and copied a huge portion of computer data to an unknown location.  This incident comes less than a month after a similar incident where their central mainframe was compromised.",
                ci.companyname
            )),
            3 => part1.push_str(&format!(
                "{} are once again reviewing their security procedures after revealing that their central mainframe has been compromised for the third time in a month, with massive amounts of confidential data being transfered away.",
                ci.companyname
            )),
            _ => part1.push_str(&format!(
                "{} have suffered yet another embarrassing security breach, today revealing that a massive quantity of private data was stolen from their central mainframe.  This system has been targetted repeatedly in recent times.",
                ci.companyname
            )),
        }

        if filetype == "SCIENTIFIC" {
            headline.push_str("Gigaquads of scientific data copied by hacker");

            part2.push_str(&format!(
                "The stolen files contained data from a massive scientific research program, which {} had been working on for the past 3 years.\nAround {} Gigaquads of data was copied before the intruder disconnected.",
                ci.companyname, totalfilesize
            ));
            part3.push_str(&format!(
                "{} will undoubtedly try to discover the corporation behind this theft.",
                ci.companyname
            ));
        } else if filetype == "CORPORATE" {
            headline.push_str("No data is secure - GigaQuads stolen in 'impossible' hack");

            part2.push_str(&format!(
                concat!(
                    "It is believed around {} Gigaquads of confidential ",
                    "corporate data was stolen by this hacker, and it was probably copied to a temporary fileserver somewhere ",
                    "on the web."
                ),
                totalfilesize
            ));

            part3.push_str(&format!(
                concat!(
                    "{} have said that they are considering all options to catch this hacker, and ",
                    "the corporation that paid him."
                ),
                ci.companyname
            ));
        } else if filetype == "CUSTOMER" {
            headline.push_str("Personal customer data copied by hacker");

            part2.push_str(&format!(
                concat!(
                    "The computer system stored hundreds of GigaQuads of personal data on all of the corporation's customers.  ",
                    "This data is now in the public domain.  Over {} Gigaquads of data was stolen."
                ),
                totalfilesize
            ));

            part3.push_str(&format!(
                concat!(
                    "{} tried to reassure the public today, and issued a statement saying they had brought ",
                    "in Federal Agents to try and find the corporation responsible."
                ),
                ci.companyname
            ));
        } else if filetype == "SOFTWARE" {
            headline.push_str("Two years of software development - stolen");

            part2.push_str(&format!(
                concat!(
                    "{} had been working on several advanced software products and this computer system ",
                    "was their primary file server.  Most of their proprietary source code has been copied - over ",
                    "{} Gigaquads of data, immediately compromising several of their planned security systems."
                ),
                ci.companyname, totalfilesize
            ));

            part3.push_str("Federal Agents have been called in to help track down the Corporation and the hacker responsible.");
        }

        //
        // Concatenate each part together and post it
        //

        let details = join_parts(&[&part1, &part2, &part3]);
        post_news(g, &headline, &details, NEWS_TYPE_STOLEN, &ci.ip, None);
    }

    /// `filetype` is SCIENTIFIC, CORPORATE, CUSTOMER or SOFTWARE.
    pub fn all_files_deleted(g: &mut Ctx, comp: &str, filetype: &str) {
        /* ===========================================================
            Structure:

            Headline
            Part1		:		Who was broken into, all files where deleted
            Part2		:		Type of data / use of computer
            Part3		:		Consequences / actions

           =========================================================== */

        let ci = comp_info(g, comp);
        let mut headline = String::new();
        let mut part1 = String::new();
        let mut part2 = String::new();
        let mut part3 = String::new();

        match ci.recent_hacks {
            0 | 1 => part1.push_str(&format!(
                "{} found themselves in the public eye today as one of their primary computer systems suffered a complete failure, resulting in a large amount of lost data.",
                ci.companyname
            )),
            2 => part1.push_str(&format!(
                "{} again found themselves in trouble as gigaquds of data where wiped from their system.  In a statement today the company claimed it had evidence that the files where deliberately deleted by a malicious hacker.",
                ci.companyname
            )),
            _ => part1.push_str(&format!(
                "{} are today boosting their security systems as their file servers suffered yet another complete failure.  The company has come under attack on several occasions in the last month by hackers who have yet to be identified.",
                ci.companyname
            )),
        }

        if filetype == "SCIENTIFIC" {
            headline.push_str("Computer failure blaimed for research loss");

            part2.push_str(&format!(
                concat!(
                    "The data deleted was the combined results of a large scale scientific test, being conducted ",
                    "by {} in order to develop new technology."
                ),
                ci.companyname
            ));

            part3.push_str(concat!(
                "In a statement today the company said that serious damage had been done and that it ",
                "could cost the company millions of dollars to perform the research again."
            ));
        } else if filetype == "CORPORATE" {
            headline.push_str("Major corporation suffers complete data loss");

            part2.push_str("The company found that masses of corporate data has been destroyed.");

            part3.push_str(concat!(
                "It is not known how much this will cost the company but there is little doubt that serious damage ",
                "has been done.  This level of data loss could not occur without serious repurcusions."
            ));
        } else if filetype == "CUSTOMER" {
            headline.push_str(&format!(
                "{} loses gigaquads of customer data due to 'malicious hackers'",
                ci.companyname
            ));

            part2.push_str(concat!(
                "The computer system that was damaged was used to hold customer records and sales data.  It is ",
                "believed that all backups of the data were also destroyed, leading the company to believe that ",
                "this was a deliberate act."
            ));

            part3.push_str(&format!(
                "{} today issued a statement saying that they were investigating the incident.",
                ci.companyname
            ));
        } else if filetype == "SOFTWARE" {
            headline.push_str("Proprietary software source code deleted by hacker");

            part2.push_str(concat!(
                "Upon investigating the damaged computer, the company found that much of their software that was ",
                "still in development had been deleted, along with all source code and back ups.  It was claimed ",
                "that the revolutionary software would dominate the market once released."
            ));

            part3.push_str(&format!(
                "{} have undoubtably suffered a large loss here, which could cost the company millions.",
                ci.companyname
            ));
        }

        //
        // Concatenate each part together and post it
        //

        let details = join_parts(&[&part1, &part2, &part3]);
        post_news(g, &headline, &details, NEWS_TYPE_DELETED, &ci.ip, None);
    }

    // Events related to people

    /// `person` = Person name; `comp` may be NULL in C++ (`None`).
    pub fn arrested(g: &mut Ctx, person: &str, comp: Option<&str>, reason: &str) {
        /* ==========================================================
            Structure:

            Headline
            Part1		:		Intro and basic overview
            part2		:		Background of person
            part3		:		World reaction / consequences

           ========================================================== */

        // The C++ read the Person's fields through the pointer; copy what the texts use.
        let (pname, uplinkrating, handle) = {
            let p = g.world().get_person(person).expect("UplinkAssert(person)");
            (
                p.name.clone(),
                p.rating.uplinkrating,
                // PORT: the C++ cast every Person to Agent here (UB for a non-agent); an empty handle is used instead.
                p.agent.as_deref().map_or_else(String::new, |a| a.handle.clone()),
            )
        };
        let comp_ip = comp.map(|c| comp_info(g, c).ip);

        let mut headline = String::new();
        let mut part1 = String::new();
        let mut part2 = String::new();
        let mut part3 = String::new();

        part1.push_str(&format!("Earlier today a man was arrested by Federal Agents for {}", reason));

        // ================================================================================================================
        // Civilian caught
        // ================================================================================================================

        if uplinkrating == 0 {
            match NumberGenerator::random_number(3) + 1 {
                1 => headline.push_str("Man arrested for Computer Crime"),
                2 => headline.push_str("Police arrest suspect in Computer fraud crime"),
                3 => headline.push_str("Federal agents arrest suspect in dawn raid"),
                _ => {}
            }

            part2.push_str(&format!(
                concat!(
                    "The man arrested, who has been named as {}, has protested his innocence ",
                    "from the start, claiming he has no knowledge of Computers at all and could not have ",
                    "committed this crime."
                ),
                pname
            ));

            match NumberGenerator::random_number(2) + 1 {
                1 => part3.push_str(concat!(
                    "The man will face court in a few months time for these crimes and could serve ",
                    " several years in jail for this crime."
                )),
                2 => part3.push_str(concat!(
                    "This case will go to court and he may face some time in jail.  ",
                    "Friends and family today began a campaign to prove his innocence, claiming ",
                    "that he does not have the knowledge or equipment necessary to complete ",
                    "the crimes he is accused of."
                )),
                _ => {}
            }
        }

        // ================================================================================================================
        // Amateur hacker caught
        // ================================================================================================================

        else if uplinkrating < 3 {
            match NumberGenerator::random_number(3) + 1 {
                1 => headline.push_str("Amateur hacker caught and arrested"),
                2 => headline.push_str("Federal Agents catch hacker red handed"),
                3 => headline.push_str("Part-time hacker brought to justice"),
                _ => {}
            }

            part2.push_str(&format!(
                concat!(
                    "The man has been named by Federal Agents as {}, and is on record with a ",
                    "history of small time computer crime.  Police who arrested him found Computer equipment and ",
                    "hardware for hacking in his house.  It is believed this man was a part-time employee of the ",
                    "underground hacker group Uplink-Corporation, where he went by the hacker alias of {}."
                ),
                pname, handle
            ));

            part3.push_str("He may not get a chance to hack again however, as he could face up to ten years in prison.");
        }

        // ================================================================================================================
        // Experienced hacker caught
        // ================================================================================================================

        else if uplinkrating < 11 {
            match NumberGenerator::random_number(3) + 1 {
                1 => headline.push_str("Experienced computer hacker finally caught"),
                2 => headline.push_str("Federal Agents catch up with underground Hacker"),
                3 => headline.push_str("High-ranked Uplink Agent arrested"),
                _ => {}
            }

            part2.push_str(&format!(
                concat!(
                    "The man's real name is {} and he is a well known player in the underground ",
                    "hacker scene.  It is believed he is a full-time freelance hacker, working for several organised ",
                    "groups including the well known Uplink-Corporation, where he went by the hacker alias of {}."
                ),
                pname, handle
            ));

            part3.push_str(concat!(
                "Police suspect this man of multiple computer crimes and as such it is believed he will serve ",
                "between ten and fifteen years inprisonment, and most likely a life ban on the use of computers."
            ));
        }

        // ================================================================================================================
        // Hacker demi-god caught
        // ================================================================================================================

        else {
            match NumberGenerator::random_number(3) + 1 {
                1 => headline.push_str("Hacker ring-leader arrested in dawn raid"),
                2 => headline.push_str("Police arrest hacker 'Demi-God'"),
                3 => headline.push_str("Federal Agents pleased with arrest of hacker leader"),
                _ => {}
            }

            part2.push_str(&format!(
                concat!(
                    "The man in question is {} and he is a well established member of the secretive ",
                    "underground hacker movement Uplink-Corporation, where he worked behind the hacker alias of {}.  ",
                    "His rating is incredibly high, suggesting a long ",
                    "history of computer crime.  Police have had a file on this many for several years but have finally ",
                    "caught him red handed."
                ),
                pname, handle
            ));

            part3.push_str(concat!(
                "Due to the nature of his history, it is believed the prosecution may push for a life sentence for ",
                "this man.  The courts are becoming increasingly hostile to hackers and they may decide to deal a ",
                "heavy punishment to this hacker as a warning to others."
            ));
        }

        //
        // Concatenate each part together and post it
        //

        let details = join_parts(&[&part1, &part2, &part3]);
        post_news(
            g,
            &headline,
            &details,
            NEWS_TYPE_ARREST,
            &pname,
            comp_ip.as_deref(),
        );
    }
}
